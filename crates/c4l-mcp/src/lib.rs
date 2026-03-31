//! Model Context Protocol (MCP) client for claw4love.
//!
//! Maps from: leak-claude-code/src/services/mcp/client.ts (3,348 lines)
//! Connects to MCP servers defined in .mcp.json, discovers tools, invokes them.

pub mod config;
pub mod client;
pub mod transport;

pub use config::McpServerConfig;
pub use client::McpClient;
