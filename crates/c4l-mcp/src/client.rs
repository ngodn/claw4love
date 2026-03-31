//! MCP client: manages connections to MCP servers and dispatches tool calls.

use crate::config::McpServerConfig;
use crate::transport::StdioTransport;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

/// Tool definition from an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    /// Which server provides this tool.
    #[serde(skip)]
    pub server_name: String,
}

/// Resource from an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip)]
    pub server_name: String,
}

/// MCP client managing multiple server connections.
pub struct McpClient {
    connections: HashMap<String, StdioTransport>,
    configs: HashMap<String, McpServerConfig>,
}

impl McpClient {
    /// Create a new empty client.
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// Load config and connect to all configured servers.
    pub async fn from_config(path: &Path) -> Result<Self> {
        let config = crate::config::load_mcp_config(path)?;
        let mut client = Self::new();

        for (name, server_config) in &config.servers {
            client.configs.insert(name.clone(), server_config.clone());

            if server_config.is_stdio() {
                match client.connect_stdio(name, server_config).await {
                    Ok(()) => info!(server = name, "connected to MCP server"),
                    Err(e) => warn!(server = name, %e, "failed to connect to MCP server"),
                }
            }
            // HTTP servers are connected on demand
        }

        Ok(client)
    }

    /// Connect to a STDIO-based MCP server.
    async fn connect_stdio(&mut self, name: &str, config: &McpServerConfig) -> Result<()> {
        let command = config.command.as_deref()
            .ok_or_else(|| anyhow::anyhow!("no command for STDIO server"))?;
        let args = config.args.clone().unwrap_or_default();
        let env = config.env.clone().unwrap_or_default();

        let mut transport = StdioTransport::spawn(command, &args, &env).await?;
        transport.initialize().await?;

        self.connections.insert(name.into(), transport);
        Ok(())
    }

    /// List tools from all connected servers.
    pub async fn list_tools(&mut self) -> Vec<McpToolDef> {
        let mut tools = Vec::new();

        let server_names: Vec<String> = self.connections.keys().cloned().collect();
        for name in server_names {
            if let Some(transport) = self.connections.get_mut(&name) {
                match transport.request("tools/list", serde_json::json!({})).await {
                    Ok(result) => {
                        if let Some(tool_list) = result.get("tools").and_then(|t| t.as_array()) {
                            for tool_json in tool_list {
                                if let Ok(mut tool) = serde_json::from_value::<McpToolDef>(tool_json.clone()) {
                                    tool.server_name = name.clone();
                                    tools.push(tool);
                                }
                            }
                        }
                    }
                    Err(e) => warn!(server = name, %e, "failed to list tools"),
                }
            }
        }

        tools
    }

    /// Execute a tool on a specific server.
    pub async fn call_tool(
        &mut self,
        server: &str,
        tool: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let transport = self.connections.get_mut(server)
            .ok_or_else(|| anyhow::anyhow!("server not connected: {server}"))?;

        debug!(server, tool, "calling MCP tool");

        transport
            .request(
                "tools/call",
                serde_json::json!({
                    "name": tool,
                    "arguments": input,
                }),
            )
            .await
    }

    /// List resources from a server.
    pub async fn list_resources(&mut self, server: &str) -> Result<Vec<McpResource>> {
        let transport = self.connections.get_mut(server)
            .ok_or_else(|| anyhow::anyhow!("server not connected: {server}"))?;

        let result = transport
            .request("resources/list", serde_json::json!({}))
            .await?;

        let mut resources = Vec::new();
        if let Some(list) = result.get("resources").and_then(|r| r.as_array()) {
            for item in list {
                if let Ok(mut resource) = serde_json::from_value::<McpResource>(item.clone()) {
                    resource.server_name = server.into();
                    resources.push(resource);
                }
            }
        }

        Ok(resources)
    }

    /// Shut down all server connections.
    pub async fn shutdown(&mut self) {
        for (name, mut transport) in self.connections.drain() {
            debug!(server = name, "shutting down MCP server");
            transport.shutdown().await.ok();
        }
    }

    /// Get the list of configured server names.
    pub fn server_names(&self) -> Vec<&str> {
        self.configs.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a server is connected.
    pub fn is_connected(&self, server: &str) -> bool {
        self.connections.contains_key(server)
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_client_is_empty() {
        let client = McpClient::new();
        assert!(client.server_names().is_empty());
    }

    #[test]
    fn mcp_tool_def_roundtrip() {
        let json = r#"{"name":"read_file","description":"Read a file","inputSchema":{"type":"object"}}"#;
        let tool: McpToolDef = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "read_file");
        assert_eq!(tool.description, "Read a file");
    }

    #[test]
    fn mcp_resource_roundtrip() {
        let json = r#"{"uri":"file:///test.txt","name":"test.txt","mimeType":"text/plain"}"#;
        let res: McpResource = serde_json::from_str(json).unwrap();
        assert_eq!(res.uri, "file:///test.txt");
        assert_eq!(res.mime_type, Some("text/plain".into()));
    }
}
