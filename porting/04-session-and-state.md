# Phase 3: Session & State — Persistence, History, Cost Tracking

## What This Phase Delivers

Durable session management (SQLite-backed), conversation history, transcript persistence, and cost tracking. Combines patterns from RTK (tracking.rs), ECC2 (session/store.rs), and the TypeScript original (sessionStorage.ts, cost-tracker.ts).

## Crates

### c4l-state

```rust
// crates/c4l-state/src/lib.rs

pub mod app_state;    // Global mutable state
pub mod session;      // Session CRUD
pub mod store;        // SQLite persistence
pub mod history;      // Conversation history
pub mod cost;         // Token cost tracking

// Re-exports
pub use app_state::AppState;
pub use session::SessionManager;
pub use store::StateStore;
```

### AppState (Global State)

Maps from: `src/state/AppStateStore.ts` (500+ lines)

```rust
// crates/c4l-state/src/app_state.rs

use c4l_types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global application state — shared via Arc<RwLock<>>
/// Maps from: TypeScript AppState (DeepImmutable wrapper)
///
/// In TypeScript this is a mutable object wrapped in DeepImmutable.
/// In Rust we use Arc<RwLock<>> for shared mutable access.
#[derive(Debug, Clone)]
pub struct AppState {
    // Conversation
    pub messages: Vec<Message>,
    pub conversation_id: uuid::Uuid,
    pub session_id: String,

    // Model & Config
    pub main_loop_model: String,
    pub verbose: bool,

    // Permissions
    pub permission_context: ToolPermissionContext,

    // Tools & Commands
    pub tool_names: Vec<String>,
    pub command_names: Vec<String>,

    // Session
    pub session_start_time: chrono::DateTime<chrono::Utc>,
    pub is_loading: bool,
    pub current_screen: Screen,

    // Tasks
    pub active_tool_use_ids: std::collections::HashSet<String>,

    // Features
    pub is_plan_mode: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Repl,
    Doctor,
    Resume,
}

pub type SharedAppState = Arc<RwLock<AppState>>;
```

### SQLite State Store

Combines RTK's `tracking.rs` and ECC2's `session/store.rs` patterns.

```rust
// crates/c4l-state/src/store.rs

use rusqlite::Connection;
use std::path::PathBuf;

/// SQLite-backed state store
/// Handles: sessions, messages, cost tracking, settings
pub struct StateStore {
    conn: Connection,
}

impl StateStore {
    /// Open or create database
    /// Location: ~/.local/share/claw4love/state.db (Linux)
    ///           ~/Library/Application Support/claw4love/state.db (macOS)
    pub fn open(path: Option<PathBuf>) -> anyhow::Result<Self>;

    /// Run schema migrations
    fn migrate(&self) -> anyhow::Result<()>;
}

// Schema (created in migrate()):
//
// CREATE TABLE sessions (
//     id TEXT PRIMARY KEY,
//     task TEXT,
//     state TEXT NOT NULL DEFAULT 'pending',
//     model TEXT,
//     pid INTEGER,
//     worktree_path TEXT,
//     worktree_branch TEXT,
//     created_at TEXT NOT NULL,
//     updated_at TEXT NOT NULL,
//     tokens_used INTEGER DEFAULT 0,
//     tool_calls INTEGER DEFAULT 0,
//     files_changed INTEGER DEFAULT 0,
//     duration_secs INTEGER DEFAULT 0,
//     cost_usd REAL DEFAULT 0.0
// );
//
// CREATE TABLE messages (
//     id INTEGER PRIMARY KEY AUTOINCREMENT,
//     session_id TEXT NOT NULL REFERENCES sessions(id),
//     type TEXT NOT NULL,  -- 'user', 'assistant', 'system', 'progress'
//     content_json TEXT NOT NULL,
//     timestamp TEXT NOT NULL
// );
//
// CREATE TABLE cost_records (
//     id INTEGER PRIMARY KEY AUTOINCREMENT,
//     session_id TEXT NOT NULL REFERENCES sessions(id),
//     model TEXT NOT NULL,
//     input_tokens INTEGER NOT NULL,
//     output_tokens INTEGER NOT NULL,
//     cache_creation_tokens INTEGER DEFAULT 0,
//     cache_read_tokens INTEGER DEFAULT 0,
//     cost_usd REAL NOT NULL,
//     timestamp TEXT NOT NULL
// );
//
// CREATE TABLE command_tracking (
//     id INTEGER PRIMARY KEY AUTOINCREMENT,
//     timestamp TEXT NOT NULL,
//     original_cmd TEXT,
//     filtered_cmd TEXT,
//     input_tokens INTEGER,
//     output_tokens INTEGER,
//     savings_pct REAL,
//     execution_time_ms INTEGER,
//     project_path TEXT
// );
//
// CREATE INDEX idx_sessions_state ON sessions(state);
// CREATE INDEX idx_messages_session ON messages(session_id);
// CREATE INDEX idx_cost_session ON cost_records(session_id);
// CREATE INDEX idx_tracking_timestamp ON command_tracking(timestamp);
```

### Session Manager

Maps from: ECC2 `session/manager.rs` + TypeScript `sessionStorage.ts`

```rust
// crates/c4l-state/src/session.rs

impl StateStore {
    // Session CRUD

    pub fn create_session(&self, task: &str, model: &str) -> anyhow::Result<Session>;

    pub fn get_session(&self, id: &str) -> anyhow::Result<Option<Session>>;

    pub fn list_sessions(
        &self,
        state: Option<SessionState>,
        limit: usize,
    ) -> anyhow::Result<Vec<Session>>;

    pub fn update_session_state(
        &self,
        id: &str,
        new_state: SessionState,
    ) -> anyhow::Result<()>;

    pub fn update_session_metrics(
        &self,
        id: &str,
        metrics: &SessionMetrics,
    ) -> anyhow::Result<()>;

    // Message persistence

    pub fn save_message(
        &self,
        session_id: &str,
        message: &Message,
    ) -> anyhow::Result<()>;

    pub fn load_messages(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Vec<Message>>;

    /// Resume a session — load messages back into AppState
    pub fn resume_session(
        &self,
        session_id: &str,
    ) -> anyhow::Result<(Session, Vec<Message>)>;
}
```

### Cost Tracker

Maps from: `src/cost-tracker.ts` + RTK's `tracking.rs`

```rust
// crates/c4l-state/src/cost.rs

/// Model pricing (USD per million tokens)
/// Updated from Anthropic pricing page
pub struct ModelPricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_write_per_mtok: f64,
    pub cache_read_per_mtok: f64,
}

pub fn get_pricing(model: &str) -> ModelPricing {
    match model {
        m if m.contains("opus") => ModelPricing {
            input_per_mtok: 15.0,
            output_per_mtok: 75.0,
            cache_write_per_mtok: 18.75,
            cache_read_per_mtok: 1.50,
        },
        m if m.contains("sonnet") => ModelPricing {
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            cache_write_per_mtok: 3.75,
            cache_read_per_mtok: 0.30,
        },
        m if m.contains("haiku") => ModelPricing {
            input_per_mtok: 0.80,
            output_per_mtok: 4.0,
            cache_write_per_mtok: 1.0,
            cache_read_per_mtok: 0.08,
        },
        _ => ModelPricing {
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            cache_write_per_mtok: 3.75,
            cache_read_per_mtok: 0.30,
        },
    }
}

impl StateStore {
    pub fn record_cost(
        &self,
        session_id: &str,
        model: &str,
        usage: &UsageData,
    ) -> anyhow::Result<f64>; // returns cost_usd

    pub fn get_session_cost(&self, session_id: &str) -> anyhow::Result<f64>;

    pub fn get_total_cost_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<f64>;

    /// RTK-style gain summary for token optimization tracking
    pub fn get_token_savings_summary(
        &self,
        days: u32,
    ) -> anyhow::Result<TokenSavingsSummary>;
}

pub struct TokenSavingsSummary {
    pub total_commands: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_saved_tokens: u64,
    pub avg_savings_pct: f64,
    pub by_command: Vec<(String, u64, u64, f64)>, // (cmd, count, saved, pct)
}
```

## Deliverables for Phase 3

1. `c4l-state` crate with SQLite store
2. Schema migrations (auto-run on first open)
3. Session CRUD (create, get, list, update state, update metrics)
4. Message persistence (save/load per session)
5. Session resume (load messages back into state)
6. Cost tracking with model-specific pricing
7. Token savings tracking (RTK integration point)
8. AppState with Arc<RwLock<>> sharing
9. Tests: session lifecycle, message round-trip, cost calculation
