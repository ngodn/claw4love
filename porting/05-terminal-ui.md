# Phase 4: Terminal UI — Ratatui REPL

## What This Phase Delivers

An interactive terminal REPL replacing React/Ink (~140 components, 80 hooks). We use ratatui + crossterm (proven in ECC2). This is NOT a 1:1 port — we build a functional REPL, not a component framework.

## Architecture

Maps from: `src/screens/REPL.tsx` (5,005 lines), `src/components/` (140 files)

```
┌─────────────────────────────────────────────┐
│ claw4love v0.1.0 | model: sonnet-4-6       │ ← Header
├─────────────────────────────────────────────┤
│                                             │
│ User: Fix the bug in auth.rs                │ ← Message list
│                                             │
│ Assistant: I'll look at the file...         │ (scrollable)
│                                             │
│ ┌─ Tool: FileRead ─────────────────────┐   │ ← Tool use
│ │ Reading src/auth.rs                   │   │
│ └───────────────────────────────────────┘   │
│                                             │
│ The issue is on line 42...                  │
│                                             │
├─────────────────────────────────────────────┤
│ > |                                         │ ← Input area
├─────────────────────────────────────────────┤
│ tokens: 1.2K in / 3.4K out | $0.02 | 2m   │ ← Status bar
└─────────────────────────────────────────────┘
```

## Core Components

```rust
// crates/c4l-tui/src/lib.rs

pub mod app;          // Application state + event loop
pub mod input;        // Text input with editing
pub mod messages;     // Message list rendering
pub mod tool_use;     // Tool invocation display
pub mod permission;   // Permission prompt dialog
pub mod status_bar;   // Status bar (cost, tokens, time)
pub mod markdown;     // Markdown rendering for terminal
pub mod spinner;      // Loading indicator

// Main entry point
pub async fn run_repl(state: SharedAppState, engine: QueryEngine) -> anyhow::Result<()>;
```

### App Event Loop

```rust
// crates/c4l-tui/src/app.rs

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc;

pub struct App {
    state: SharedAppState,
    engine: QueryEngine,
    input: InputState,
    scroll_offset: usize,
    mode: AppMode,
    query_events_rx: mpsc::Receiver<QueryEvent>,
    query_events_tx: mpsc::Sender<QueryEvent>,
}

pub enum AppMode {
    Input,          // Normal text input
    Streaming,      // Receiving assistant response
    PermissionPrompt(PermissionRequest),  // Waiting for user approval
    Scrolling,      // Scrolling through message history
}

impl App {
    /// Main event loop — runs until exit
    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        loop {
            // 1. Render current state
            terminal.draw(|frame| self.render(frame))?;

            // 2. Handle events (keyboard, query events, resize)
            tokio::select! {
                // Keyboard input
                Ok(true) = tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(50))) => {
                    if let Event::Key(key) = event::read()? {
                        if self.handle_key(key).await? == Action::Quit {
                            break;
                        }
                    }
                }
                // Query engine events (streaming text, tool results, etc.)
                Some(event) = self.query_events_rx.recv() => {
                    self.handle_query_event(event).await?;
                }
            }
        }
        Ok(())
    }
}
```

### Text Input

```rust
// crates/c4l-tui/src/input.rs

/// Multi-line text input with basic editing
/// Maps from: useTextInput() hook in TypeScript
pub struct InputState {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub history: Vec<String>,       // Previous inputs
    pub history_index: Option<usize>,
}

impl InputState {
    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction;
    pub fn content(&self) -> String;    // Join lines
    pub fn clear(&mut self);
    pub fn is_empty(&self) -> bool;
}

pub enum InputAction {
    Continue,           // Keep editing
    Submit(String),     // Enter pressed — submit message
    Cancel,             // Escape pressed
    ScrollUp,           // Page up
    ScrollDown,         // Page down
}
```

### Message Rendering

```rust
// crates/c4l-tui/src/messages.rs

/// Render a message list with proper formatting
/// Maps from: src/components/Message.tsx + ToolUse.tsx + ToolResult.tsx
pub fn render_messages(
    messages: &[Message],
    area: Rect,
    buf: &mut Buffer,
    scroll_offset: usize,
    streaming_text: Option<&str>,
);

/// Render markdown content for terminal
/// Uses pulldown-cmark for parsing, manual ANSI styling
/// Maps from: TypeScript's marked + highlight.js rendering
pub fn render_markdown(content: &str, width: u16) -> Vec<Line<'_>>;
```

### Permission Prompt

```rust
// crates/c4l-tui/src/permission.rs

/// Permission dialog shown when a tool needs approval
/// Maps from: src/components/PermissionPrompt.tsx
pub struct PermissionRequest {
    pub tool_name: String,
    pub description: String,
    pub input_summary: String,
}

pub enum PermissionResponse {
    Allow,
    AlwaysAllow,    // Add to always-allow rules
    Deny,
}

pub fn render_permission_dialog(
    request: &PermissionRequest,
    area: Rect,
    buf: &mut Buffer,
);
```

## Key Differences from TypeScript

| TypeScript (React/Ink) | Rust (ratatui) |
|------------------------|----------------|
| Component tree with JSX | Direct buffer rendering |
| React hooks for state | Arc<RwLock<AppState>> |
| Virtual DOM diffing | Full terminal redraw |
| ~140 components | ~8 render functions |
| 80 React hooks | tokio::select! event loop |
| Ink's built-in layout | ratatui's Layout::default() |

## Deliverables for Phase 4

1. `c4l-tui` crate with ratatui REPL
2. Multi-line text input with history
3. Message list with scrolling
4. Streaming text display (real-time as tokens arrive)
5. Tool use/result rendering
6. Permission prompt dialog
7. Status bar (model, tokens, cost, elapsed time)
8. Basic markdown rendering (bold, italic, code blocks, headers)
9. Ctrl+C interrupt handling
10. Clean terminal restore on exit
