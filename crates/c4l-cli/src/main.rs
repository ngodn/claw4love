use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod session;

#[derive(Parser)]
#[command(name = "claw4love", version, about = "Claude Code CLI — reimagined in Rust")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Verbose output (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Model to use
    #[arg(short, long, global = true)]
    model: Option<String>,

    /// Run a single prompt (non-interactive)
    #[arg(short, long)]
    prompt: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current configuration
    Config,
    /// Show version and build info
    Version,
    /// List recent sessions
    Sessions {
        /// Max sessions to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Resume a previous session
    Resume {
        /// Session ID (prefix match)
        session_id: String,
    },
    /// Show token cost for current or all sessions
    Cost {
        /// Show costs for all sessions
        #[arg(short, long)]
        all: bool,
    },
    /// Run environment diagnostics
    Doctor,
    /// Log in with your Claude subscription (Pro/Max/Team/Enterprise)
    Login {
        /// Pre-fill email address
        #[arg(short, long)]
        email: Option<String>,
    },
    /// Log out and clear stored credentials
    Logout,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity
    let filter = match cli.verbose {
        0 => "c4l=warn",
        1 => "c4l=info",
        2 => "c4l=debug",
        _ => "c4l=trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| filter.into()),
        )
        .init();

    // Load config
    let cwd = std::env::current_dir().ok();
    let config = c4l_config::C4lConfig::load(cwd.as_deref())?;

    // Override model from CLI if provided
    let model = cli
        .model
        .unwrap_or_else(|| config.model.default_model.clone());

    match cli.command {
        Some(Commands::Config) => cmd_config(&config),
        Some(Commands::Version) => cmd_version(&config),
        Some(Commands::Sessions { limit }) => cmd_sessions(limit)?,
        Some(Commands::Resume { session_id }) => cmd_resume(&session_id)?,
        Some(Commands::Cost { all }) => cmd_cost(all)?,
        Some(Commands::Doctor) => cmd_doctor(&config),
        Some(Commands::Login { email }) => cmd_login(email.as_deref()).await?,
        Some(Commands::Logout) => cmd_logout()?,
        None => {
            // Non-interactive mode with --prompt
            if let Some(prompt) = cli.prompt {
                session::run_oneshot(&config, &model, &prompt).await?;
            } else {
                // Interactive REPL
                session::run_interactive(&config, &model).await?;
            }
        }
    }

    Ok(())
}

fn cmd_config(config: &c4l_config::C4lConfig) {
    match toml::to_string_pretty(config) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("Failed to serialize config: {e}"),
    }
}

fn cmd_version(config: &c4l_config::C4lConfig) {
    println!("claw4love {}", env!("CARGO_PKG_VERSION"));
    println!("model: {}", config.model.default_model);
    println!("api: {}", config.api_base_url());
}

fn cmd_sessions(limit: usize) -> Result<()> {
    let store = c4l_state::StateStore::open(None)?;
    let sessions = store.list_sessions(None, limit)?;

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    println!("{:<12} {:<10} {:<30} {:>10} {:>8}", "ID", "STATE", "TASK", "TOKENS", "COST");
    println!("{}", "-".repeat(75));
    for s in &sessions {
        println!(
            "{:<12} {:<10} {:<30} {:>10} {:>8}",
            &s.id[..12.min(s.id.len())],
            format!("{:?}", s.state),
            if s.task.len() > 28 {
                format!("{}...", &s.task[..25])
            } else {
                s.task.clone()
            },
            s.metrics.tokens_used,
            format!("${:.2}", s.metrics.cost_usd),
        );
    }

    Ok(())
}

fn cmd_resume(session_id: &str) -> Result<()> {
    let store = c4l_state::StateStore::open(None)?;

    // Find session by prefix
    let sessions = store.list_sessions(None, 100)?;
    let matching: Vec<_> = sessions.iter().filter(|s| s.id.starts_with(session_id)).collect();

    match matching.len() {
        0 => println!("No session found matching '{session_id}'"),
        1 => {
            let session = &matching[0];
            let messages = store.load_messages(&session.id)?;
            println!("Session: {} ({:?})", &session.id[..12], session.state);
            println!("Task: {}", session.task);
            println!("Messages: {}", messages.len());
            println!("\n(Resume with interactive mode coming soon)");
        }
        n => {
            println!("Ambiguous: {n} sessions match '{session_id}'");
            for s in matching {
                println!("  {} - {}", &s.id[..12], s.task);
            }
        }
    }

    Ok(())
}

fn cmd_cost(all: bool) -> Result<()> {
    let store = c4l_state::StateStore::open(None)?;

    if all {
        let total = store.get_total_cost_since(
            chrono::Utc::now() - chrono::Duration::days(30),
        )?;
        println!("Total cost (last 30 days): ${:.4}", total);
    } else {
        let sessions = store.list_sessions(None, 1)?;
        if let Some(s) = sessions.first() {
            println!("Last session: {}", &s.id[..12]);
            println!("  Tokens: {}", s.metrics.tokens_used);
            println!("  Cost: ${:.4}", s.metrics.cost_usd);
        } else {
            println!("No sessions found.");
        }
    }

    Ok(())
}

fn cmd_doctor(config: &c4l_config::C4lConfig) {
    println!("claw4love doctor\n");

    // Check auth
    print!("  Auth: ");
    match c4l_api::oauth::resolve_auth(config) {
        c4l_api::AuthMethod::ApiKey(_) => println!("API key"),
        c4l_api::AuthMethod::OAuth(token) => {
            let sub = token.subscription_type.as_deref().unwrap_or("unknown");
            let expired = if token.is_expired() { " (EXPIRED)" } else { "" };
            println!("OAuth subscription ({sub}){expired}");
        }
        c4l_api::AuthMethod::None => {
            println!("NONE - run 'claw4love login' or set ANTHROPIC_API_KEY");
        }
    }

    // Check model
    println!("  Model: {}", config.model.default_model);
    println!("  API URL: {}", config.api_base_url());

    // Check ripgrep
    print!("  ripgrep (rg): ");
    match std::process::Command::new("rg").arg("--version").output() {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout);
            println!("{}", version.lines().next().unwrap_or("installed"));
        }
        _ => println!("NOT FOUND - Grep tool will not work"),
    }

    // Check git
    print!("  git: ");
    match std::process::Command::new("git").arg("--version").output() {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout);
            println!("{}", version.trim());
        }
        _ => println!("NOT FOUND"),
    }

    // Check CLAUDE.md
    let cwd = std::env::current_dir().unwrap_or_default();
    let memory_files = c4l_plugins::load_memory_files(&cwd);
    println!("  CLAUDE.md files: {}", memory_files.len());
    for f in &memory_files {
        println!("    {:?}: {}", f.scope, f.path.display());
    }

    // Check plugins
    let plugin_dirs: Vec<PathBuf> = vec![];
    let plugins = c4l_plugins::discover_plugins(&plugin_dirs);
    println!("  Plugins: {}", plugins.len());

    // Check skills
    let skill_dirs: Vec<PathBuf> = vec![];
    let skills = c4l_plugins::discover_skills(&skill_dirs);
    println!("  Skills: {}", skills.len());

    println!("\nDone.");
}

async fn cmd_login(email: Option<&str>) -> Result<()> {
    use c4l_api::oauth;

    // Check if already logged in
    if let Ok(Some(token)) = oauth::load_credentials() {
        if !token.is_expired() {
            let sub = token.subscription_type.as_deref().unwrap_or("unknown");
            println!("Already logged in (subscription: {sub}).");
            println!("Run 'claw4love logout' first to re-authenticate.");
            return Ok(());
        }
        println!("Token expired, re-authenticating...");
    }

    // Generate PKCE challenge
    let pkce = oauth::PkceChallenge::generate();

    // Start local callback server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://localhost:{port}/callback");

    // Build authorization URL
    let auth_url = oauth::build_authorize_url(&pkce, &redirect_uri, email);

    println!("Opening browser for authentication...\n");
    println!("If the browser doesn't open, visit this URL:\n{auth_url}\n");

    // Try to open browser
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(&auth_url).spawn().ok();
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&auth_url).spawn().ok();
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd").args(["/C", "start", &auth_url]).spawn().ok();

    println!("Waiting for authorization...");

    // Wait for the callback
    let (stream, _addr) = listener.accept().await?;
    let mut buf = vec![0u8; 4096];
    stream.readable().await?;
    let n = stream.try_read(&mut buf).unwrap_or(0);
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the authorization code from the callback URL
    let code = request
        .lines()
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|path| {
            path.split('?')
                .nth(1)
                .and_then(|query| {
                    query.split('&').find_map(|param| {
                        param.strip_prefix("code=").map(String::from)
                    })
                })
        })
        .ok_or_else(|| anyhow::anyhow!("no authorization code in callback"))?;

    // Send success response to browser
    let response = format!(
        "HTTP/1.1 302 Found\r\nLocation: {}\r\nConnection: close\r\n\r\n",
        oauth::SUCCESS_URL,
    );
    stream.writable().await?;
    stream.try_write(response.as_bytes()).ok();

    // Exchange code for tokens
    let http = reqwest::Client::new();
    let token = oauth::exchange_code(
        &http,
        &code,
        &redirect_uri,
        &pkce.verifier,
        &pkce.state,
    )
    .await?;

    // Save credentials
    oauth::save_credentials(&token)?;

    let sub = token.subscription_type.as_deref().unwrap_or("active");
    println!("\nLogged in successfully (subscription: {sub}).");
    println!("You can now use claw4love without an API key.");

    Ok(())
}

fn cmd_logout() -> Result<()> {
    c4l_api::oauth::clear_credentials()?;
    println!("Logged out. Credentials cleared.");
    Ok(())
}
