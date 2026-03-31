//! MCP server configuration types.
//!
//! Loaded from .mcp.json in the project root or ~/.claude/.mcp.json

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level .mcp.json structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub servers: HashMap<String, McpServerConfig>,
}

/// Configuration for a single MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to start a STDIO-based server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Arguments for the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Transport type: "stdio" (default) or "http".
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transport_type: Option<String>,
    /// URL for HTTP-based servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Environment variables to set for the server process.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

impl McpServerConfig {
    /// Whether this server uses STDIO transport.
    pub fn is_stdio(&self) -> bool {
        self.command.is_some()
            && self.transport_type.as_deref() != Some("http")
    }

    /// Whether this server uses HTTP transport.
    pub fn is_http(&self) -> bool {
        self.url.is_some()
            || self.transport_type.as_deref() == Some("http")
    }
}

/// Load MCP configuration from a .mcp.json file.
pub fn load_mcp_config(path: &Path) -> Result<McpConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: McpConfig = serde_json::from_str(&content)?;
    Ok(config)
}

/// Discover .mcp.json files from project and user directories.
pub fn find_mcp_configs(project_root: &Path) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // Project root
    let project_mcp = project_root.join(".mcp.json");
    if project_mcp.exists() {
        paths.push(project_mcp);
    }

    // User global
    if let Some(home) = dirs::home_dir() {
        let user_mcp = home.join(".claude").join(".mcp.json");
        if user_mcp.exists() {
            paths.push(user_mcp);
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mcp_config() {
        let json = r#"{
            "mcpServers": {
                "github": {
                    "command": "npx",
                    "args": ["@modelcontextprotocol/server-github"]
                },
                "exa": {
                    "type": "http",
                    "url": "https://mcp.exa.ai/mcp"
                }
            }
        }"#;

        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.servers.len(), 2);

        let github = &config.servers["github"];
        assert!(github.is_stdio());
        assert!(!github.is_http());
        assert_eq!(github.command.as_deref(), Some("npx"));

        let exa = &config.servers["exa"];
        assert!(exa.is_http());
        assert!(!exa.is_stdio());
        assert_eq!(exa.url.as_deref(), Some("https://mcp.exa.ai/mcp"));
    }

    #[test]
    fn load_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(".mcp.json");
        std::fs::write(&file, r#"{"mcpServers": {"test": {"command": "echo"}}}"#).unwrap();

        let config = load_mcp_config(&file).unwrap();
        assert_eq!(config.servers.len(), 1);
    }

    #[test]
    fn server_with_env() {
        let json = r#"{
            "mcpServers": {
                "custom": {
                    "command": "my-server",
                    "env": {"API_KEY": "secret123"}
                }
            }
        }"#;

        let config: McpConfig = serde_json::from_str(json).unwrap();
        let server = &config.servers["custom"];
        assert_eq!(
            server.env.as_ref().unwrap().get("API_KEY").unwrap(),
            "secret123"
        );
    }
}
