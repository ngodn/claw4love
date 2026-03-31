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
    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key).unwrap_or(HeaderValue::from_static("")),
        );
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
            metadata: None,
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
