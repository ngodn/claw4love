//! Session management, SQLite persistence, and cost tracking.
//!
//! Combines patterns from:
//! - RTK tracking.rs (SQLite command tracking)
//! - ECC2 session/store.rs (session state machine)
//! - leak-claude-code sessionStorage.ts + cost-tracker.ts

pub mod store;
pub mod cost;
pub mod app_state;

pub use store::StateStore;
pub use app_state::{AppState, SharedAppState};
