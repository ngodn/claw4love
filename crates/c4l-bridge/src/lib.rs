//! IDE bridge for claw4love.
//!
//! Maps from: leak-claude-code/src/bridge/ (16 files, ~5K lines)
//!
//! This is a thin initial layer. Full IDE bridge implementation will grow
//! as VS Code / JetBrains extension compatibility is needed.

pub mod protocol;

pub use protocol::{BridgeMessage, BridgeEvent};
