//! Message types for the conversation system.
//!
//! Maps from: leak-claude-code/src/types/message.ts
//! Verified fields: UserMessage, AssistantMessage, SystemMessage, ProgressMessage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Top-level message — discriminated union via `type` field.
///
/// In TypeScript: `type Message = UserMessage | AssistantMessage | SystemMessage | ProgressMessage`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "system")]
    System(SystemMessage),
    #[serde(rename = "progress")]
    Progress(ProgressMessage),
}

/// User message in the conversation.
///
/// Verified fields from: leak-claude-code/src/types/message.ts lines 80-105
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    pub message: UserMessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_meta: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_compact_summary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<MessageOrigin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageContent {
    pub role: String,
    pub content: ContentBlock,
}

/// Content can be a plain string or an array of typed blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlock {
    Text(String),
    Blocks(Vec<ContentBlockParam>),
}

/// Individual content block parameter — text, image, tool_use, tool_result.
///
/// Maps from Anthropic SDK ContentBlockParam variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockParam {
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

/// Assistant message — response from the LLM.
///
/// Verified fields from: leak-claude-code/src/types/message.ts lines 48-65
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    /// The raw API response — kept as Value since BetaMessage structure may evolve.
    pub message: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_api_error_message: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_error: Option<String>,
}

/// System message with subtype discrimination.
///
/// Verified from: leak-claude-code/src/types/message.ts lines 108-140
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    pub subtype: SystemMessageSubtype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<SystemMessageLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemMessageSubtype {
    Informational,
    ApiError,
    LocalCommand,
    ToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemMessageLevel {
    Info,
    Warning,
    Error,
}

/// Progress message — emitted during tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    pub tool_use_id: String,
    pub parent_tool_use_id: String,
    /// Progress payload varies by tool type — kept generic.
    pub data: serde_json::Value,
}

/// Origin of a message in the system.
///
/// Verified from: leak-claude-code/src/types/message.ts line 5
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageOrigin {
    Agent,
    Teammate,
    Command,
    System,
    Hook,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_roundtrip() {
        let msg = Message::User(UserMessage {
            uuid: Uuid::new_v4(),
            timestamp: Utc::now(),
            message: UserMessageContent {
                role: "user".into(),
                content: ContentBlock::Text("Fix the bug in auth.rs".into()),
            },
            is_meta: None,
            is_compact_summary: None,
            origin: None,
        });

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        match deserialized {
            Message::User(u) => {
                assert_eq!(u.message.role, "user");
                match u.message.content {
                    ContentBlock::Text(t) => assert_eq!(t, "Fix the bug in auth.rs"),
                    _ => panic!("expected text content"),
                }
            }
            _ => panic!("expected user message"),
        }
    }

    #[test]
    fn system_message_roundtrip() {
        let msg = Message::System(SystemMessage {
            uuid: Uuid::new_v4(),
            timestamp: Utc::now(),
            subtype: SystemMessageSubtype::Informational,
            content: Some("Session started".into()),
            level: Some(SystemMessageLevel::Info),
            tool_use_id: None,
        });

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        match deserialized {
            Message::System(s) => {
                assert_eq!(s.content.unwrap(), "Session started");
            }
            _ => panic!("expected system message"),
        }
    }

    #[test]
    fn tool_use_content_block_roundtrip() {
        let block = ContentBlockParam::ToolUse {
            id: "toolu_123".into(),
            name: "BashTool".into(),
            input: serde_json::json!({"command": "ls -la"}),
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
        assert!(json.contains("\"name\":\"BashTool\""));

        let back: ContentBlockParam = serde_json::from_str(&json).unwrap();
        match back {
            ContentBlockParam::ToolUse { name, .. } => assert_eq!(name, "BashTool"),
            _ => panic!("expected tool_use"),
        }
    }

    #[test]
    fn message_with_blocks_content() {
        let msg = Message::User(UserMessage {
            uuid: Uuid::new_v4(),
            timestamp: Utc::now(),
            message: UserMessageContent {
                role: "user".into(),
                content: ContentBlock::Blocks(vec![
                    ContentBlockParam::Text {
                        text: "Check this file".into(),
                    },
                    ContentBlockParam::ToolResult {
                        tool_use_id: "toolu_456".into(),
                        content: serde_json::json!("file contents here"),
                        is_error: Some(false),
                    },
                ]),
            },
            is_meta: None,
            is_compact_summary: None,
            origin: Some(MessageOrigin::Agent),
        });

        let json = serde_json::to_string_pretty(&msg).unwrap();
        let back: Message = serde_json::from_str(&json).unwrap();

        match back {
            Message::User(u) => match u.message.content {
                ContentBlock::Blocks(blocks) => assert_eq!(blocks.len(), 2),
                _ => panic!("expected blocks"),
            },
            _ => panic!("expected user"),
        }
    }
}
