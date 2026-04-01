//! Session orchestration: wire engine, tools, TUI, commands, plugins together.
//!
//! This is where all the crates come together into a working conversation.

use anyhow::Result;
use c4l_api::oauth::{self, AuthMethod};
use c4l_api::{AnthropicClient, ApiConfig};
use c4l_commands::CommandRegistry;
use c4l_engine::engine::EngineConfig;
use c4l_engine::events::QueryEvent;
use c4l_engine::QueryEngine;
use c4l_plugins::load_memory_files;
use c4l_state::{AppState, StateStore};
use tokio::sync::mpsc;
use tracing::info;

/// Build the system prompt from memory files and skill context.
fn build_system_prompt(project_root: &std::path::Path) -> String {
    let mut parts = vec![
        "You are an interactive agent that helps users with software engineering tasks.".to_string(),
        "Use the tools available to you to assist the user.".to_string(),
    ];

    // Load CLAUDE.md memory files
    let memory_files = load_memory_files(project_root);
    if !memory_files.is_empty() {
        let memory_prompt = c4l_plugins::memory::build_memory_prompt(&memory_files);
        parts.push(memory_prompt);
    }

    parts.join("\n\n")
}

/// Build an AnthropicClient — either directly (API key) or via session bootstrap (OAuth).
///
/// For OAuth/subscription users: spawns a real Claude Code CLI session through a
/// transparent proxy to capture the exact headers, metadata, and session ID.
/// This ensures 100% compatibility with Anthropic's API requirements.
async fn build_client(config: &c4l_config::C4lConfig, model: &str) -> Result<AnthropicClient> {
    match oauth::resolve_auth(config) {
        AuthMethod::ApiKey(key) => {
            info!("authenticating with API key");
            Ok(AnthropicClient::new(ApiConfig::new(key, model.to_string())))
        }
        AuthMethod::OAuth(_) => {
            info!("OAuth detected, bootstrapping session via Claude Code CLI");
            eprintln!("Bootstrapping session via Claude Code CLI...");
            let session = c4l_api::bootstrap_session().await?;
            eprintln!("Session ready (model: {}, id: {})", session.model, &session.session_id[..8.min(session.session_id.len())]);
            Ok(AnthropicClient::from_captured_session(&session))
        }
        AuthMethod::None => {
            anyhow::bail!(
                "No authentication found.\n\
                 Run 'claw4love login' to authenticate with your Claude subscription,\n\
                 or set ANTHROPIC_API_KEY for API key access."
            )
        }
    }
}

/// Run a single prompt without the TUI (non-interactive / pipe mode).
pub async fn run_oneshot(
    config: &c4l_config::C4lConfig,
    model: &str,
    prompt: &str,
) -> Result<()> {
    let client = build_client(config, model).await?;
    let cwd = std::env::current_dir()?;

    let system_prompt = build_system_prompt(&cwd);
    let tool_registry = c4l_engine::ToolRegistry::new(); // no tools in oneshot for now

    let engine_config = EngineConfig {
        max_turns: 1,
        system_prompt,
        append_system_prompt: None,
    };

    let mut engine = QueryEngine::new(client, engine_config, tool_registry);

    let (event_tx, mut event_rx) = mpsc::channel::<QueryEvent>(256);

    // Run engine in background
    let prompt_owned = prompt.to_string();
    let engine_handle = tokio::spawn(async move {
        engine.submit(prompt_owned, event_tx).await
    });

    // Print events to stdout
    while let Some(event) = event_rx.recv().await {
        match event {
            QueryEvent::TextDelta(text) => print!("{text}"),
            QueryEvent::TurnComplete { .. } => {
                println!();
                break;
            }
            QueryEvent::Error(msg) => {
                eprintln!("\nError: {msg}");
                break;
            }
            _ => {}
        }
    }

    engine_handle.await??;
    Ok(())
}

/// Run the interactive REPL session.
pub async fn run_interactive(
    config: &c4l_config::C4lConfig,
    model: &str,
) -> Result<()> {
    let client = build_client(config, model).await?;
    let cwd = std::env::current_dir()?;

    // Open state store
    let store = StateStore::open(None)?;
    let session = store.create_session("interactive", model)?;
    store.update_session_state(&session.id, &c4l_types::SessionState::Running)?;
    info!(session_id = %session.id, model, "starting interactive session");

    // Build system prompt
    let system_prompt = build_system_prompt(&cwd);

    // Build tool registry (engine's simple version for now)
    let tool_registry = c4l_engine::ToolRegistry::new();

    // Build engine
    let engine_config = EngineConfig {
        max_turns: 100,
        system_prompt,
        append_system_prompt: None,
    };
    let mut engine = QueryEngine::new(client, engine_config, tool_registry);

    // Build command registry
    let commands = CommandRegistry::default();

    // Build shared app state
    let app_state = AppState::shared(session.id.clone(), model.to_string());

    // Channels: user input -> engine, engine events -> TUI
    let (user_tx, mut user_rx) = mpsc::channel::<String>(16);
    let (query_event_tx, query_event_rx) = mpsc::channel::<QueryEvent>(256);

    // Spawn the TUI
    let tui_state = app_state.clone();
    let tui_model = model.to_string();
    let tui_session_id = session.id.clone();
    let tui_handle = tokio::spawn(async move {
        c4l_tui::run_repl(
            tui_state,
            tui_model,
            tui_session_id,
            user_tx,
            query_event_rx,
        )
        .await
    });

    // Main loop: receive user messages, dispatch to engine or commands
    let store_for_loop = StateStore::open(None)?;
    while let Some(user_input) = user_rx.recv().await {
        // Check for slash commands
        if let Some(result) = commands.dispatch(&user_input, &app_state) {
            match result {
                Ok(c4l_commands::CommandResult::Text(text)) => {
                    let _ = query_event_tx
                        .send(QueryEvent::TextDelta(format!("{text}\n")))
                        .await;
                    let _ = query_event_tx
                        .send(QueryEvent::TurnComplete {
                            stop_reason: c4l_engine::StopReason::EndTurn,
                        })
                        .await;
                }
                Ok(c4l_commands::CommandResult::Prompt { prompt, .. }) => {
                    // Send the generated prompt to the engine
                    engine.submit(prompt, query_event_tx.clone()).await?;
                }
                Ok(c4l_commands::CommandResult::Exit) => {
                    break;
                }
                Ok(c4l_commands::CommandResult::None) => {}
                Err(e) => {
                    let _ = query_event_tx
                        .send(QueryEvent::Error(format!("Command error: {e}")))
                        .await;
                }
            }
            continue;
        }

        // Regular message: send to engine
        // Save user message to store
        let user_msg = c4l_types::Message::User(c4l_types::UserMessage {
            uuid: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            message: c4l_types::UserMessageContent {
                role: "user".into(),
                content: c4l_types::ContentBlock::Text(user_input.clone()),
            },
            is_meta: None,
            is_compact_summary: None,
            origin: None,
        });
        store_for_loop.save_message(&session.id, &user_msg).ok();

        // Submit to engine
        engine.submit(user_input, query_event_tx.clone()).await?;
    }

    // Mark session complete
    store.update_session_state(&session.id, &c4l_types::SessionState::Completed).ok();

    // Wait for TUI to exit
    tui_handle.abort();

    Ok(())
}
