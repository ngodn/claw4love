//! Terminal UI for claw4love: ratatui REPL.
//!
//! Replaces React/Ink (~140 components, 80 hooks) with ~8 render functions
//! and a tokio::select! event loop.

pub mod app;
pub mod input;
pub mod render;

pub use app::run_repl;
