//! Tool trait and implementations for claw4love.
//!
//! Maps from: leak-claude-code/src/Tool.ts (interface) + src/tools/ (implementations)

pub mod traits;
pub mod registry;
pub mod bash;
pub mod file_read;
pub mod file_edit;
pub mod file_write;
pub mod glob;
pub mod grep;

pub use traits::Tool;
pub use registry::ToolRegistry;
