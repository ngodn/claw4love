//! Application event loop: keyboard input, query events, rendering.

use crate::input::{InputAction, InputState};
use crate::render::{self, DisplayMessage};
use c4l_engine::events::{QueryEvent, StopReason};
use c4l_state::SharedAppState;
use anyhow::Result;
use crossterm::{
    event::{self, Event},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

/// Application mode.
#[derive(Debug, Clone)]
pub enum AppMode {
    Input,
    Streaming,
    PermissionPrompt {
        tool_name: String,
        description: String,
    },
}

/// The TUI application.
pub struct App {
    #[allow(dead_code)]
    state: SharedAppState,
    input: InputState,
    messages: Vec<DisplayMessage>,
    streaming_text: String,
    scroll_offset: usize,
    mode: AppMode,
    model: String,
    session_id: String,
    input_tokens: u64,
    output_tokens: u64,
    cost_usd: f64,
    start_time: std::time::Instant,
    /// Channel for sending user messages out to whoever drives the engine.
    user_tx: mpsc::Sender<String>,
    /// Channel for receiving query events from the engine.
    query_rx: mpsc::Receiver<QueryEvent>,
    should_quit: bool,
}

impl App {
    pub fn new(
        state: SharedAppState,
        model: String,
        session_id: String,
        user_tx: mpsc::Sender<String>,
        query_rx: mpsc::Receiver<QueryEvent>,
    ) -> Self {
        Self {
            state,
            input: InputState::new(),
            messages: Vec::new(),
            streaming_text: String::new(),
            scroll_offset: 0,
            mode: AppMode::Input,
            model,
            session_id,
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: 0.0,
            start_time: std::time::Instant::now(),
            user_tx,
            query_rx,
            should_quit: false,
        }
    }

    /// Render the full UI layout.
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: header (1) | messages (flex) | input (3) | status (1)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // header
                Constraint::Min(5),    // messages
                Constraint::Length(3), // input
                Constraint::Length(1), // status bar
            ])
            .split(area);

        render::render_header(frame, chunks[0], &self.model, &self.session_id);

        let streaming = if self.streaming_text.is_empty() {
            None
        } else {
            Some(self.streaming_text.as_str())
        };
        render::render_messages(frame, chunks[1], &self.messages, self.scroll_offset, streaming);
        render::render_input(frame, chunks[2], &self.input, &self.mode);
        render::render_status_bar(
            frame,
            chunks[3],
            &self.model,
            self.input_tokens,
            self.output_tokens,
            self.cost_usd,
            self.start_time.elapsed(),
        );

        // Permission prompt overlay
        if let AppMode::PermissionPrompt { ref tool_name, ref description } = self.mode {
            render::render_permission_prompt(frame, area, tool_name, description);
        }
    }

    /// Handle a keyboard event.
    async fn handle_key(&mut self, key_event: event::KeyEvent) -> Result<()> {
        // Permission prompt mode: only y/a/n keys matter
        if let AppMode::PermissionPrompt { .. } = &self.mode {
            match key_event.code {
                event::KeyCode::Char('y') | event::KeyCode::Char('Y') => {
                    self.mode = AppMode::Input;
                    // TODO: send permission response
                }
                event::KeyCode::Char('a') | event::KeyCode::Char('A') => {
                    self.mode = AppMode::Input;
                    // TODO: send always-allow response
                }
                event::KeyCode::Char('n') | event::KeyCode::Char('N') | event::KeyCode::Esc => {
                    self.mode = AppMode::Input;
                    // TODO: send deny response
                }
                _ => {}
            }
            return Ok(());
        }

        let action = self.input.handle_key(key_event);
        match action {
            InputAction::Submit(text) => {
                self.messages.push(DisplayMessage::User(text.clone()));
                self.scroll_offset = 0; // auto-scroll
                self.mode = AppMode::Streaming;
                self.streaming_text.clear();
                let _ = self.user_tx.send(text).await;
            }
            InputAction::Interrupt => {
                self.should_quit = true;
            }
            InputAction::Cancel => {
                if matches!(self.mode, AppMode::Streaming) {
                    // TODO: abort current query
                    self.mode = AppMode::Input;
                }
            }
            InputAction::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_add(5);
            }
            InputAction::ScrollDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
            }
            InputAction::Continue => {}
        }
        Ok(())
    }

    /// Handle a query event from the engine.
    async fn handle_query_event(&mut self, event: QueryEvent) {
        match event {
            QueryEvent::TextDelta(text) => {
                self.streaming_text.push_str(&text);
            }
            QueryEvent::ThinkingDelta(_) => {
                // Could show thinking indicator; skip for now
            }
            QueryEvent::ToolUseStart { name, .. } => {
                self.messages.push(DisplayMessage::ToolUse {
                    name,
                    input_summary: String::new(),
                });
            }
            QueryEvent::ToolInputDelta { .. } => {}
            QueryEvent::ToolResult { name, result, is_error, .. } => {
                let summary = result
                    .as_str()
                    .map(|s| {
                        if s.len() > 200 {
                            format!("{}...", &s[..200])
                        } else {
                            s.to_string()
                        }
                    })
                    .unwrap_or_else(|| "(structured output)".into());

                self.messages.push(DisplayMessage::ToolResult {
                    name,
                    summary,
                    is_error,
                });
            }
            QueryEvent::Usage(usage) => {
                self.input_tokens += usage.input_tokens;
                self.output_tokens += usage.output_tokens;
            }
            QueryEvent::TurnComplete { stop_reason } => {
                // Flush streaming text into a message
                if !self.streaming_text.is_empty() {
                    let text = std::mem::take(&mut self.streaming_text);
                    self.messages.push(DisplayMessage::Assistant(text));
                }
                self.mode = AppMode::Input;
                self.scroll_offset = 0;

                if stop_reason == StopReason::MaxTokens {
                    self.messages.push(DisplayMessage::System(
                        "(max tokens reached)".into(),
                    ));
                }
            }
            QueryEvent::Error(msg) => {
                self.messages.push(DisplayMessage::System(format!("Error: {msg}")));
                self.mode = AppMode::Input;
            }
        }
    }

    /// Run the main event loop.
    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if self.should_quit {
                break;
            }

            tokio::select! {
                // Poll keyboard (crossterm is blocking, use spawn_blocking)
                result = tokio::task::spawn_blocking(|| {
                    if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                        event::read().ok()
                    } else {
                        None
                    }
                }) => {
                    if let Ok(Some(Event::Key(key))) = result {
                        self.handle_key(key).await?;
                    }
                }
                // Receive query events
                Some(event) = self.query_rx.recv() => {
                    self.handle_query_event(event).await;
                }
            }
        }
        Ok(())
    }
}

/// Entry point: set up terminal, run app, restore terminal.
pub async fn run_repl(
    state: SharedAppState,
    model: String,
    session_id: String,
    user_tx: mpsc::Sender<String>,
    query_rx: mpsc::Receiver<QueryEvent>,
) -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let mut app = App::new(state, model, session_id, user_tx, query_rx);
    let result = app.run(&mut terminal).await;

    // Restore terminal
    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_mode_default_is_input() {
        let (user_tx, _) = mpsc::channel(1);
        let (_, query_rx) = mpsc::channel(1);
        let state = c4l_state::AppState::shared("test".into(), "model".into());
        let app = App::new(state, "sonnet".into(), "sess-123".into(), user_tx, query_rx);
        assert!(matches!(app.mode, AppMode::Input));
    }
}
