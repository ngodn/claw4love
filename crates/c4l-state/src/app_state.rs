//! Global application state shared across the application.
//!
//! Maps from: leak-claude-code/src/state/AppStateStore.ts

use c4l_types::*;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state, accessible from engine, TUI, and tools.
pub type SharedAppState = Arc<RwLock<AppState>>;

/// Global application state.
///
/// Maps from: TypeScript AppState (wrapped in DeepImmutable).
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

    // Session
    pub session_start_time: chrono::DateTime<chrono::Utc>,
    pub is_loading: bool,
    pub current_screen: Screen,

    // Active operations
    pub active_tool_use_ids: HashSet<String>,

    // Mode
    pub is_plan_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Repl,
    Doctor,
    Resume,
}

impl AppState {
    pub fn new(session_id: String, model: String) -> Self {
        Self {
            messages: Vec::new(),
            conversation_id: uuid::Uuid::new_v4(),
            session_id,
            main_loop_model: model,
            verbose: false,
            permission_context: ToolPermissionContext::default(),
            session_start_time: chrono::Utc::now(),
            is_loading: false,
            current_screen: Screen::Repl,
            active_tool_use_ids: HashSet::new(),
            is_plan_mode: false,
        }
    }

    pub fn shared(session_id: String, model: String) -> SharedAppState {
        Arc::new(RwLock::new(Self::new(session_id, model)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_defaults() {
        let state = AppState::new("test-session".into(), "claude-sonnet-4-6".into());
        assert!(state.messages.is_empty());
        assert_eq!(state.current_screen, Screen::Repl);
        assert!(!state.is_loading);
        assert!(!state.is_plan_mode);
    }

    #[tokio::test]
    async fn shared_state_read_write() {
        let state = AppState::shared("test".into(), "model".into());

        {
            let mut w = state.write().await;
            w.is_loading = true;
        }

        {
            let r = state.read().await;
            assert!(r.is_loading);
        }
    }
}
