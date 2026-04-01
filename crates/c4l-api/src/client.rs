//! Anthropic Messages API HTTP client.
//!
//! Handles: request building, streaming, retry with backoff.

use crate::error::ApiError;
use crate::sse::SseLineParser;
use crate::types::*;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// The main API client.
pub struct AnthropicClient {
    http: reqwest::Client,
    config: ApiConfig,
    retry_policy: RetryPolicy,
}

impl AnthropicClient {
    pub fn new(config: ApiConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
            retry_policy: RetryPolicy::default(),
        }
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    pub fn config(&self) -> &ApiConfig {
        &self.config
    }

    /// Build the required HTTP headers for the Anthropic API.
    ///
    /// API key auth: sends x-api-key header.
    /// OAuth auth: sends Authorization: Bearer header + oauth beta.
    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        match &self.config.auth {
            crate::types::ApiAuth::ApiKey(key) => {
                if let Ok(val) = HeaderValue::from_str(key) {
                    headers.insert("x-api-key", val);
                }
            }
            crate::types::ApiAuth::OAuth(token) => {
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
                    headers.insert("authorization", val);
                }
            }
        }

        headers.insert(
            "anthropic-version",
            HeaderValue::from_str(&self.config.api_version)
                .unwrap_or(HeaderValue::from_static("2023-06-01")),
        );
        if !self.config.betas.is_empty() {
            let betas = self.config.betas.join(",");
            if let Ok(val) = HeaderValue::from_str(&betas) {
                headers.insert("anthropic-beta", val);
            }
        }

        // Required headers for OAuth/subscription requests
        if matches!(self.config.auth, crate::types::ApiAuth::OAuth(_)) {
            headers.insert(
                "user-agent",
                HeaderValue::from_static("claude-cli/2.1.87 (external, claude-vscode)"),
            );
            headers.insert("x-app", HeaderValue::from_static("cli"));
            headers.insert(
                "anthropic-dangerous-direct-browser-access",
                HeaderValue::from_static("true"),
            );
        }

        headers
    }

    /// Build the request body.
    fn build_request(
        &self,
        messages: &[ApiMessage],
        system: Option<&str>,
        tools: &[ApiToolDef],
        stream: bool,
    ) -> MessagesRequest {
        MessagesRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages: messages.to_vec(),
            system: system.map(String::from),
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.to_vec())
            },
            stream: if stream { Some(true) } else { None },
            metadata: match &self.config.auth {
                crate::types::ApiAuth::OAuth(_) => {
                    // OAuth requires metadata.user_id with device_id, account_uuid, session_id
                    // device_id: from ~/.claude/config.json userID (shared with Claude Code CLI)
                    // account_uuid: from OAuth credentials
                    // session_id: random UUID per session
                    let device_id = crate::oauth::get_device_id();
                    let account_uuid = crate::oauth::get_account_uuid();
                    Some(serde_json::json!({
                        "user_id": serde_json::json!({
                            "device_id": device_id,
                            "account_uuid": account_uuid,
                            "session_id": uuid::Uuid::new_v4().to_string(),
                        }).to_string()
                    }))
                }
                _ => None,
            },
            thinking: match &self.config.auth {
                crate::types::ApiAuth::OAuth(_) => Some(ThinkingConfig {
                    thinking_type: "disabled".into(),
                    budget_tokens: None,
                }),
                _ => None,
            },
            temperature: match &self.config.auth {
                // temperature:1 required when thinking is disabled
                crate::types::ApiAuth::OAuth(_) => Some(1.0),
                _ => None,
            },
        }
    }

    /// Send a non-streaming request to the Messages API.
    pub async fn create_message(
        &self,
        messages: &[ApiMessage],
        system: Option<&str>,
        tools: &[ApiToolDef],
    ) -> Result<MessageResponse, ApiError> {
        let body = self.build_request(messages, system, tools, false);

        for attempt in 0..=self.retry_policy.max_retries {
            debug!(attempt, "sending messages request");

            let result = self
                .http
                .post(&self.config.messages_url())
                .headers(self.headers())
                .json(&body)
                .send()
                .await;

            match result {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if status == 200 {
                        let response: MessageResponse = resp.json().await?;
                        return Ok(response);
                    }

                    let body_text = resp.text().await.unwrap_or_default();
                    let err = ApiError::from_status(status, &body_text);

                    if err.is_retryable() && attempt < self.retry_policy.max_retries {
                        let delay = self.retry_policy.delay_ms(attempt);
                        warn!(attempt, delay_ms = delay, %err, "retryable error, backing off");
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(err);
                }
                Err(e) => {
                    let err = ApiError::Network(e);
                    if err.is_retryable() && attempt < self.retry_policy.max_retries {
                        let delay = self.retry_policy.delay_ms(attempt);
                        warn!(attempt, delay_ms = delay, %err, "network error, retrying");
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        unreachable!("retry loop should have returned")
    }

    /// Stream a request to the Messages API, sending events through a channel.
    ///
    /// This is the primary method used by the query engine. Events are sent
    /// through the mpsc channel as they arrive from the SSE stream.
    pub async fn stream_messages(
        &self,
        messages: &[ApiMessage],
        system: Option<&str>,
        tools: &[ApiToolDef],
        event_tx: mpsc::Sender<Result<StreamEvent, ApiError>>,
    ) -> Result<(), ApiError> {
        let body = self.build_request(messages, system, tools, true);

        if tracing::enabled!(tracing::Level::DEBUG) {
            if let Ok(json) = serde_json::to_string_pretty(&body) {
                debug!(body = %json, "request body");
            }
        }

        for attempt in 0..=self.retry_policy.max_retries {
            debug!(attempt, "starting streaming request");

            let result = self
                .http
                .post(&self.config.messages_url())
                .headers(self.headers())
                .json(&body)
                .send()
                .await;

            match result {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if status != 200 {
                        let body_text = resp.text().await.unwrap_or_default();
                        tracing::error!(status, body = %body_text, url = %self.config.messages_url(), "API error response");
                        let err = ApiError::from_status(status, &body_text);

                        if err.is_retryable() && attempt < self.retry_policy.max_retries {
                            let delay = self.retry_policy.delay_ms(attempt);
                            warn!(attempt, delay_ms = delay, %err, "retryable error, backing off");
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                            continue;
                        }
                        return Err(err);
                    }

                    // Read the SSE stream
                    let mut parser = SseLineParser::new();
                    let mut byte_stream = resp;

                    while let Ok(Some(chunk)) = byte_stream.chunk().await {
                        let text = String::from_utf8_lossy(&chunk);
                        let events = parser.feed(&text);
                        for event_result in events {
                            if event_tx.send(event_result).await.is_err() {
                                // Receiver dropped, stop streaming
                                return Ok(());
                            }
                        }
                    }

                    return Ok(());
                }
                Err(e) => {
                    let err = ApiError::Network(e);
                    if err.is_retryable() && attempt < self.retry_policy.max_retries {
                        let delay = self.retry_policy.delay_ms(attempt);
                        warn!(attempt, delay_ms = delay, %err, "network error, retrying");
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        unreachable!("retry loop should have returned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_contain_required_fields() {
        let client = AnthropicClient::new(ApiConfig::new("sk-test-key".into(), "claude-sonnet-4-6".into()));
        let headers = client.headers();

        assert_eq!(
            headers.get("x-api-key").unwrap().to_str().unwrap(),
            "sk-test-key"
        );
        assert!(headers.get("authorization").is_none());
        assert_eq!(
            headers.get("anthropic-version").unwrap().to_str().unwrap(),
            "2023-06-01"
        );
        assert_eq!(
            headers.get("content-type").unwrap().to_str().unwrap(),
            "application/json"
        );
    }

    #[test]
    fn headers_include_betas() {
        let mut config = ApiConfig::new("sk-test".into(), "claude-sonnet-4-6".into());
        config.betas = vec!["extended-thinking-2025-04-14".into()];

        let client = AnthropicClient::new(config);
        let headers = client.headers();

        assert_eq!(
            headers.get("anthropic-beta").unwrap().to_str().unwrap(),
            "extended-thinking-2025-04-14"
        );
    }

    #[test]
    fn oauth_headers() {
        let config = ApiConfig::with_oauth("oauth-token-123".into(), "claude-sonnet-4-6".into());
        let client = AnthropicClient::new(config);
        let headers = client.headers();

        assert_eq!(
            headers.get("authorization").unwrap().to_str().unwrap(),
            "Bearer oauth-token-123"
        );
        assert!(headers.get("x-api-key").is_none());
        // OAuth config auto-includes the oauth beta
        assert!(headers.get("anthropic-beta").unwrap().to_str().unwrap().contains("oauth-2025-04-20"));
    }

    #[test]
    fn build_request_with_tools() {
        let client = AnthropicClient::new(ApiConfig::new("sk-test".into(), "claude-sonnet-4-6".into()));

        let messages = vec![ApiMessage {
            role: "user".into(),
            content: ApiContent::Text("hello".into()),
        }];

        let tools = vec![ApiToolDef {
            name: "BashTool".into(),
            description: "Execute shell commands".into(),
            input_schema: serde_json::json!({"type": "object", "properties": {"command": {"type": "string"}}}),
        }];

        let req = client.build_request(&messages, Some("Be helpful"), &tools, true);

        assert!(req.stream.unwrap());
        assert_eq!(req.tools.as_ref().unwrap().len(), 1);
        assert_eq!(req.system.as_deref(), Some("Be helpful"));
    }

    #[test]
    fn build_request_without_tools() {
        let client = AnthropicClient::new(ApiConfig::new("sk-test".into(), "claude-sonnet-4-6".into()));

        let messages = vec![ApiMessage {
            role: "user".into(),
            content: ApiContent::Text("hello".into()),
        }];

        let req = client.build_request(&messages, None, &[], false);

        assert!(req.stream.is_none());
        assert!(req.tools.is_none());
        assert!(req.system.is_none());
    }
}
