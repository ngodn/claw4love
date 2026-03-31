//! Events emitted by the query engine during a conversation turn.

use c4l_api::UsageData;
use serde::{Deserialize, Serialize};

/// Events emitted during query processing.
///
/// Sent through an mpsc channel to decouple the engine from the UI.
/// The TUI or any other consumer can process these at its own pace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryEvent {
    /// Streaming text content from the assistant.
    TextDelta(String),

    /// Streaming thinking content (extended thinking mode).
    ThinkingDelta(String),

    /// A tool_use block was detected. The engine will execute it.
    ToolUseStart { id: String, name: String },

    /// Tool input is being streamed (partial JSON).
    ToolInputDelta { id: String, partial_json: String },

    /// Tool execution completed.
    ToolResult {
        id: String,
        name: String,
        result: serde_json::Value,
        is_error: bool,
    },

    /// Token usage update.
    Usage(UsageData),

    /// A turn in the conversation completed.
    TurnComplete { stop_reason: StopReason },

    /// Error occurred (may or may not be fatal).
    Error(String),
}

/// Why the current turn ended.
///
/// Maps from: Anthropic API stop_reason field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model finished its response naturally.
    EndTurn,
    /// The model hit the max_tokens limit.
    MaxTokens,
    /// The model wants to use a tool (engine will loop).
    ToolUse,
    /// A stop sequence was encountered.
    StopSequence,
}

impl StopReason {
    /// Parse from the API's stop_reason string.
    pub fn from_api(s: &str) -> Self {
        match s {
            "end_turn" => Self::EndTurn,
            "max_tokens" => Self::MaxTokens,
            "tool_use" => Self::ToolUse,
            "stop_sequence" => Self::StopSequence,
            _ => Self::EndTurn,
        }
    }

    /// Whether this stop reason means the engine should continue (execute tools and loop).
    pub fn should_continue(&self) -> bool {
        *self == Self::ToolUse
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_reason_parsing() {
        assert_eq!(StopReason::from_api("end_turn"), StopReason::EndTurn);
        assert_eq!(StopReason::from_api("tool_use"), StopReason::ToolUse);
        assert_eq!(StopReason::from_api("max_tokens"), StopReason::MaxTokens);
        assert_eq!(StopReason::from_api("unknown"), StopReason::EndTurn);
    }

    #[test]
    fn tool_use_should_continue() {
        assert!(StopReason::ToolUse.should_continue());
        assert!(!StopReason::EndTurn.should_continue());
        assert!(!StopReason::MaxTokens.should_continue());
    }
}
