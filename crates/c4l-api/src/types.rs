//! API request/response types for the Anthropic Messages API.
//!
//! Based on: https://docs.anthropic.com/en/api/messages
//! Verified against: leak-claude-code/src/services/api/claude.ts

use serde::{Deserialize, Serialize};

/// Configuration for the API client.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub api_version: String,
    pub betas: Vec<String>,
}

impl ApiConfig {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com".into(),
            model,
            max_tokens: 16384,
            api_version: "2023-06-01".into(),
            betas: vec![],
        }
    }

    /// Endpoint URL for the messages API.
    pub fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url)
    }
}

/// Retry policy with exponential backoff.
///
/// Maps from: leak-claude-code/src/services/api/errors.ts retry logic
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_factor: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Calculate delay for a given attempt (0-indexed).
    pub fn delay_ms(&self, attempt: u32) -> u64 {
        let delay = self.initial_delay_ms as f64 * self.backoff_factor.powi(attempt as i32);
        (delay as u64).min(self.max_delay_ms)
    }
}

// -- Request types --

/// A message in the API request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: ApiContent,
}

/// Content can be a string or an array of content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiContent {
    Text(String),
    Blocks(Vec<ApiContentBlock>),
}

/// Individual content block in a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Tool definition sent to the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Full request body for POST /v1/messages.
#[derive(Debug, Clone, Serialize)]
pub struct MessagesRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ApiToolDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// -- Response types --

/// Full (non-streaming) response from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ResponseContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: UsageData,
}

/// Content block in the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "server_tool_use")]
    ServerToolUse { id: String, name: String },
}

/// Token usage data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageData {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u64>,
}

// -- SSE streaming event types --

/// Server-Sent Event from the streaming API.
///
/// Based on: https://docs.anthropic.com/en/api/messages-streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageResponse },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ResponseContentBlock,
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: ContentDelta },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },

    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaData,
        usage: UsageData,
    },

    #[serde(rename = "message_stop")]
    MessageStop {},

    #[serde(rename = "ping")]
    Ping {},

    #[serde(rename = "error")]
    Error { error: StreamError },
}

/// Delta within a content block.
///
/// Verified from: leak-claude-code/src/services/api/claude.ts lines 1979-2296
/// Delta types: text_delta, input_json_delta, thinking_delta, signature_delta,
///              citations_delta, connector_text_delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },

    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },

    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },

    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },

    #[serde(rename = "citations_delta")]
    CitationsDelta { citations: serde_json::Value },

    #[serde(rename = "connector_text_delta")]
    ConnectorTextDelta { connector_text: String },
}

/// Delta at the message level (stop reason update).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
}

/// Error within the stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_config_default_url() {
        let config = ApiConfig::new("sk-test".into(), "claude-sonnet-4-6".into());
        assert_eq!(config.messages_url(), "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn retry_policy_backoff() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay_ms(0), 1000);
        assert_eq!(policy.delay_ms(1), 2000);
        assert_eq!(policy.delay_ms(2), 4000);
        assert_eq!(policy.delay_ms(10), 30000); // capped at max
    }

    #[test]
    fn messages_request_serialize() {
        let req = MessagesRequest {
            model: "claude-sonnet-4-6".into(),
            max_tokens: 4096,
            messages: vec![ApiMessage {
                role: "user".into(),
                content: ApiContent::Text("Hello".into()),
            }],
            system: Some("You are helpful.".into()),
            tools: None,
            stream: Some(true),
            metadata: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"model\":\"claude-sonnet-4-6\""));
    }

    #[test]
    fn stream_event_text_delta_roundtrip() {
        let json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let event: StreamEvent = serde_json::from_str(json).unwrap();
        match event {
            StreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    ContentDelta::TextDelta { text } => assert_eq!(text, "Hello"),
                    _ => panic!("expected text_delta"),
                }
            }
            _ => panic!("expected content_block_delta"),
        }
    }

    #[test]
    fn stream_event_tool_use_start() {
        let json = r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_123","name":"BashTool","input":{}}}"#;
        let event: StreamEvent = serde_json::from_str(json).unwrap();
        match event {
            StreamEvent::ContentBlockStart { index, content_block } => {
                assert_eq!(index, 1);
                match content_block {
                    ResponseContentBlock::ToolUse { name, .. } => assert_eq!(name, "BashTool"),
                    _ => panic!("expected tool_use"),
                }
            }
            _ => panic!("expected content_block_start"),
        }
    }

    #[test]
    fn stream_event_message_stop() {
        let json = r#"{"type":"message_stop"}"#;
        let event: StreamEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, StreamEvent::MessageStop {}));
    }

    #[test]
    fn usage_data_with_cache() {
        let json = r#"{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":20,"cache_read_input_tokens":80}"#;
        let usage: UsageData = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.cache_read_input_tokens, Some(80));
    }

    #[test]
    fn content_delta_input_json() {
        let json = r#"{"type":"input_json_delta","partial_json":"{\"command\":"}"#;
        let delta: ContentDelta = serde_json::from_str(json).unwrap();
        match delta {
            ContentDelta::InputJsonDelta { partial_json } => {
                assert!(partial_json.contains("command"));
            }
            _ => panic!("expected input_json_delta"),
        }
    }
}
