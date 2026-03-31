//! MCP transport layer: STDIO and HTTP.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{debug, warn};

/// JSON-RPC message for MCP protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

impl JsonRpcMessage {
    pub fn request(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::Value::Number(id.into())),
            method: Some(method.into()),
            params: Some(params),
            result: None,
            error: None,
        }
    }
}

/// STDIO transport: communicates with a child process via stdin/stdout.
pub struct StdioTransport {
    child: Child,
    next_id: u64,
}

impl StdioTransport {
    /// Spawn a child process for the MCP server.
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
    ) -> Result<Self> {
        debug!(command, ?args, "spawning MCP server");

        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        for (k, v) in env {
            cmd.env(k, v);
        }

        let child = cmd.spawn()?;
        Ok(Self { child, next_id: 1 })
    }

    /// Send a JSON-RPC request and wait for the response.
    pub async fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let msg = JsonRpcMessage::request(id, method, params);
        let json = serde_json::to_string(&msg)?;

        // Write to stdin
        let stdin = self.child.stdin.as_mut()
            .ok_or_else(|| anyhow::anyhow!("child stdin not available"))?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read from stdout until we get a response with matching id
        let stdout = self.child.stdout.as_mut()
            .ok_or_else(|| anyhow::anyhow!("child stdout not available"))?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).await?;
            if bytes == 0 {
                anyhow::bail!("MCP server closed stdout");
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(response) = serde_json::from_str::<JsonRpcMessage>(line) {
                if response.id == Some(serde_json::Value::Number(id.into())) {
                    if let Some(error) = response.error {
                        anyhow::bail!("MCP error: {}", error);
                    }
                    return Ok(response.result.unwrap_or(serde_json::Value::Null));
                }
            }
        }
    }

    /// Send the initialize handshake.
    pub async fn initialize(&mut self) -> Result<serde_json::Value> {
        self.request(
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "claw4love",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
        .await
    }

    /// Shut down the server process.
    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.request("shutdown", serde_json::json!({})).await;
        self.child.kill().await.ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_rpc_request_format() {
        let msg = JsonRpcMessage::request(1, "tools/list", serde_json::json!({}));
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"tools/list\""));
    }
}
