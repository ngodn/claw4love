//! Session bootstrap: spawn a real Claude Code CLI session through a transparent proxy,
//! capture all headers/metadata, then take over the session.
//!
//! Flow:
//! 1. Start a local TCP proxy on a random port
//! 2. Proxy forwards all traffic to api.anthropic.com transparently
//! 3. Spawn `claude --print -p "hi"` with ANTHROPIC_BASE_URL pointed to the proxy
//! 4. Proxy captures: Authorization, anthropic-beta, User-Agent, Session-Id, metadata, body structure
//! 5. Claude gets a real response (session created on Anthropic's side)
//! 6. Kill claude process, return captured session info
//! 7. claw4love continues using the exact same credentials

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tracing::{debug, info, warn};

/// Everything captured from a real Claude Code session.
#[derive(Debug, Clone)]
pub struct CapturedSession {
    /// All HTTP headers from the POST request
    pub headers: HashMap<String, String>,
    /// The full request body (JSON)
    pub body: String,
    /// Parsed fields for convenience
    pub authorization: String,
    pub anthropic_beta: String,
    pub user_agent: String,
    pub session_id: String,
    pub model: String,
    pub metadata: serde_json::Value,
    pub thinking: serde_json::Value,
}

impl CapturedSession {
    /// Build a reqwest HeaderMap from the captured headers.
    pub fn to_header_map(&self) -> reqwest::header::HeaderMap {
        let mut map = reqwest::header::HeaderMap::new();
        for (k, v) in &self.headers {
            let key = k.to_lowercase();
            // Skip hop-by-hop headers
            if matches!(
                key.as_str(),
                "host" | "connection" | "content-length" | "accept-encoding" | "transfer-encoding"
            ) {
                continue;
            }
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                map.insert(name, val);
            }
        }
        map
    }
}

/// Bootstrap a session by spawning Claude Code CLI through a transparent proxy.
///
/// Returns the captured session info that claw4love can use for subsequent requests.
pub async fn bootstrap_session() -> Result<CapturedSession> {
    // Check that claude CLI is available
    let claude_path = which_claude()?;
    info!(?claude_path, "found Claude Code CLI");

    // Start transparent proxy
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let proxy_port = listener.local_addr()?.port();
    info!(proxy_port, "proxy listening");

    let captured: Arc<Mutex<Option<CapturedSession>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();

    // Spawn proxy handler
    let proxy_handle = tokio::spawn(async move {
        if let Ok((client_stream, _)) = listener.accept().await {
            // First request might be a HEAD health check, skip it
            handle_proxy_connection(client_stream, &captured_clone).await.ok();

            // Accept the actual POST request
            if captured_clone.lock().unwrap().is_none() {
                if let Ok((client_stream2, _)) = listener.accept().await {
                    handle_proxy_connection(client_stream2, &captured_clone).await.ok();
                }
            }
        }
    });

    // Spawn claude CLI
    info!("spawning Claude Code CLI for session bootstrap");
    let mut claude_proc = tokio::process::Command::new(&claude_path)
        .args(["--print", "-p", "say hi briefly"])
        .env("ANTHROPIC_BASE_URL", format!("http://127.0.0.1:{proxy_port}"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn claude CLI")?;

    // Wait for proxy to capture the request (with timeout)
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(30), proxy_handle).await;

    // Kill claude process
    claude_proc.kill().await.ok();

    match timeout {
        Ok(Ok(())) => {}
        Ok(Err(e)) => warn!(%e, "proxy task error"),
        Err(_) => warn!("proxy capture timed out"),
    }

    // Extract captured session
    let session = captured
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed to capture session from Claude Code CLI"))?;

    info!(
        model = %session.model,
        session_id = %session.session_id,
        beta = %session.anthropic_beta,
        "session bootstrapped successfully"
    );

    Ok(session)
}

/// Handle one proxy connection: read request, forward to real API, capture headers.
async fn handle_proxy_connection(
    mut client: tokio::net::TcpStream,
    captured: &Arc<Mutex<Option<CapturedSession>>>,
) -> Result<()> {
    // Read the full HTTP request
    let _buf = Vec::<u8>::new();
    let mut reader = BufReader::new(&mut client);

    // Read request line
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;
    let request_line = request_line.trim().to_string();

    debug!(request_line, "proxy received request");

    // Read headers
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let line = line.trim().to_string();
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(": ") {
            if key.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().unwrap_or(0);
            }
            headers.insert(key.to_string(), value.to_string());
        }
    }

    // Skip HEAD requests (health check)
    if request_line.starts_with("HEAD") {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        client.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    // Read body
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await?;
    }
    let body_str = String::from_utf8_lossy(&body).to_string();

    // Parse the request path
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/v1/messages");

    // Forward to real API
    let real_url = format!("https://api.anthropic.com{path}");
    debug!(real_url, content_length, "forwarding to Anthropic API");

    let http = reqwest::Client::new();
    let mut req_builder = http.post(&real_url);

    // Forward all headers
    for (k, v) in &headers {
        let key = k.to_lowercase();
        if matches!(key.as_str(), "host" | "connection" | "accept-encoding") {
            continue;
        }
        req_builder = req_builder.header(k.as_str(), v.as_str());
    }

    let resp = req_builder
        .body(body.clone())
        .send()
        .await
        .context("failed to forward request to Anthropic")?;

    let resp_status = resp.status();
    let resp_headers = resp.headers().clone();
    let resp_body = resp.bytes().await.unwrap_or_default();

    debug!(status = %resp_status, resp_len = resp_body.len(), "got response from Anthropic");

    // Send response back to claude process
    let mut response = format!("HTTP/1.1 {}\r\n", resp_status);
    for (k, v) in &resp_headers {
        if let Ok(val) = v.to_str() {
            response.push_str(&format!("{}: {}\r\n", k, val));
        }
    }
    response.push_str(&format!("content-length: {}\r\n", resp_body.len()));
    response.push_str("\r\n");

    client.write_all(response.as_bytes()).await.ok();
    client.write_all(&resp_body).await.ok();

    // Parse and capture the session info (only for POST requests that succeeded)
    if request_line.contains("POST") && resp_status.is_success() || resp_status.as_u16() == 200 {
        let authorization = headers
            .get("Authorization")
            .or_else(|| headers.get("authorization"))
            .cloned()
            .unwrap_or_default();
        let anthropic_beta = headers
            .get("anthropic-beta")
            .cloned()
            .unwrap_or_default();
        let user_agent = headers
            .get("User-Agent")
            .or_else(|| headers.get("user-agent"))
            .cloned()
            .unwrap_or_default();
        let session_id = headers
            .get("X-Claude-Code-Session-Id")
            .or_else(|| headers.get("x-claude-code-session-id"))
            .cloned()
            .unwrap_or_default();

        // Parse body JSON for model and metadata
        let body_json: serde_json::Value =
            serde_json::from_str(&body_str).unwrap_or(serde_json::Value::Null);
        let model = body_json
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("claude-sonnet-4-6")
            .to_string();
        let metadata = body_json
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let thinking = body_json
            .get("thinking")
            .cloned()
            .unwrap_or(serde_json::json!({"type": "disabled"}));

        let session = CapturedSession {
            headers,
            body: body_str,
            authorization,
            anthropic_beta,
            user_agent,
            session_id,
            model,
            metadata,
            thinking,
        };

        *captured.lock().unwrap() = Some(session);
        info!("captured Claude Code session");
    } else if request_line.contains("POST") {
        // Capture even on error so we have the headers
        let authorization = headers
            .get("Authorization")
            .or_else(|| headers.get("authorization"))
            .cloned()
            .unwrap_or_default();
        let anthropic_beta = headers
            .get("anthropic-beta")
            .cloned()
            .unwrap_or_default();
        let user_agent = headers
            .get("User-Agent")
            .or_else(|| headers.get("user-agent"))
            .cloned()
            .unwrap_or_default();
        let session_id = headers
            .get("X-Claude-Code-Session-Id")
            .or_else(|| headers.get("x-claude-code-session-id"))
            .cloned()
            .unwrap_or_default();

        let body_json: serde_json::Value =
            serde_json::from_str(&body_str).unwrap_or(serde_json::Value::Null);
        let model = body_json
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("claude-sonnet-4-6")
            .to_string();
        let metadata = body_json
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let thinking = body_json
            .get("thinking")
            .cloned()
            .unwrap_or(serde_json::json!({"type": "disabled"}));

        let session = CapturedSession {
            headers,
            body: body_str,
            authorization,
            anthropic_beta,
            user_agent,
            session_id,
            model,
            metadata,
            thinking,
        };

        *captured.lock().unwrap() = Some(session);
        warn!(status = %resp_status, "captured session (API returned error, headers still captured)");
    }

    Ok(())
}

/// Find the claude CLI binary.
fn which_claude() -> Result<std::path::PathBuf> {
    let output = std::process::Command::new("which")
        .arg("claude")
        .output()
        .context("failed to run 'which claude'")?;

    if !output.status.success() {
        anyhow::bail!(
            "Claude Code CLI not found. Install it first: npm install -g @anthropic-ai/claude-code"
        );
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(std::path::PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_session_to_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".into(), "Bearer test-token".into());
        headers.insert("anthropic-beta".into(), "oauth-2025-04-20".into());
        headers.insert("Content-Type".into(), "application/json".into());
        headers.insert("Host".into(), "localhost".into()); // should be filtered

        let session = CapturedSession {
            headers,
            body: String::new(),
            authorization: "Bearer test-token".into(),
            anthropic_beta: "oauth-2025-04-20".into(),
            user_agent: "claude-cli/test".into(),
            session_id: "test-session".into(),
            model: "claude-sonnet-4-6".into(),
            metadata: serde_json::Value::Null,
            thinking: serde_json::json!({"type": "disabled"}),
        };

        let header_map = session.to_header_map();
        assert!(header_map.get("authorization").is_some());
        assert!(header_map.get("anthropic-beta").is_some());
        assert!(header_map.get("host").is_none()); // filtered
    }
}
