//! Bridge protocol types for IDE communication.
//!
//! Maps from: leak-claude-code/src/bridge/types.ts + bridgeMessaging.ts

use serde::{Deserialize, Serialize};

/// Message sent from the CLI to the IDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BridgeMessage {
    /// Request permission approval from IDE.
    #[serde(rename = "permission_request")]
    PermissionRequest {
        id: String,
        tool_name: String,
        description: String,
        input_summary: String,
    },

    /// Stream text to the IDE.
    #[serde(rename = "text_delta")]
    TextDelta { text: String },

    /// Tool execution started.
    #[serde(rename = "tool_start")]
    ToolStart { id: String, name: String },

    /// Tool execution completed.
    #[serde(rename = "tool_result")]
    ToolResult {
        id: String,
        name: String,
        is_error: bool,
        summary: String,
    },

    /// Session state update.
    #[serde(rename = "state_update")]
    StateUpdate {
        session_id: String,
        model: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
    },
}

/// Event received from the IDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BridgeEvent {
    /// User submitted a message from the IDE.
    #[serde(rename = "user_message")]
    UserMessage { text: String },

    /// Permission response from the IDE.
    #[serde(rename = "permission_response")]
    PermissionResponse {
        id: String,
        allowed: bool,
        always_allow: bool,
    },

    /// IDE requests session info.
    #[serde(rename = "get_state")]
    GetState,

    /// IDE requests to abort current query.
    #[serde(rename = "abort")]
    Abort,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_message_roundtrip() {
        let msg = BridgeMessage::PermissionRequest {
            id: "req-1".into(),
            tool_name: "Bash".into(),
            description: "Execute shell command".into(),
            input_summary: "git status".into(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"permission_request\""));

        let back: BridgeMessage = serde_json::from_str(&json).unwrap();
        match back {
            BridgeMessage::PermissionRequest { tool_name, .. } => {
                assert_eq!(tool_name, "Bash");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn bridge_event_roundtrip() {
        let event = BridgeEvent::PermissionResponse {
            id: "req-1".into(),
            allowed: true,
            always_allow: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        let back: BridgeEvent = serde_json::from_str(&json).unwrap();
        match back {
            BridgeEvent::PermissionResponse { allowed, .. } => assert!(allowed),
            _ => panic!("wrong variant"),
        }
    }
}
