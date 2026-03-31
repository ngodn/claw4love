//! Query engine: the core conversation loop.
//!
//! Maps from: leak-claude-code/src/QueryEngine.ts (~46K lines)
//! We implement the essential: stream API, detect tool_use, execute, loop back.
//!
//! The 46K lines in TypeScript include React rendering, plugin hooks, telemetry,
//! and feature flags. The core loop is ~2K lines of logic, which is what we port.

pub mod engine;
pub mod events;
pub mod tool_registry;

pub use engine::QueryEngine;
pub use events::{QueryEvent, StopReason};
pub use tool_registry::ToolRegistry;
