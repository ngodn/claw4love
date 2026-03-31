//! Slash command system for claw4love.
//!
//! Maps from: leak-claude-code/src/commands.ts + src/commands/ (85 directories)

pub mod traits;
pub mod registry;
pub mod builtins;

pub use traits::{Command, CommandResult};
pub use registry::CommandRegistry;
