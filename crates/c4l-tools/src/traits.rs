//! The core Tool trait that all tools implement.
//!
//! Maps from: leak-claude-code/src/Tool.ts Tool<Input, Output, Progress> interface

use async_trait::async_trait;
use c4l_types::{PermissionResult, ToolInputSchema, ValidationResult};
use serde_json::Value;

/// Result of a tool execution, sent back to the API as tool_result.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub data: Value,
    pub is_error: bool,
    pub display: Option<String>,
}

impl ToolResult {
    pub fn success(data: Value) -> Self {
        Self { data, is_error: false, display: None }
    }

    pub fn success_text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            data: Value::String(text.clone()),
            is_error: false,
            display: Some(text),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            data: Value::String(msg.clone()),
            is_error: true,
            display: Some(msg),
        }
    }
}

/// Context provided to tools during execution.
pub struct ToolUseContext {
    pub working_directory: std::path::PathBuf,
    pub permission_context: c4l_types::ToolPermissionContext,
    pub verbose: bool,
}

/// The core tool trait. Every tool implements this.
///
/// Maps from: TypeScript Tool<Input, Output, Progress> in Tool.ts
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name as sent to the Anthropic API.
    fn name(&self) -> &str;

    /// Alternative names the model might use.
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }

    /// JSON Schema for the tool's input parameters.
    fn input_schema(&self) -> ToolInputSchema;

    /// Description sent to the API for this tool.
    fn description(&self) -> &str;

    /// Text injected into the system prompt when this tool is available.
    fn prompt(&self) -> &str {
        ""
    }

    /// Execute the tool with the given input.
    async fn call(&self, input: Value, context: &ToolUseContext) -> anyhow::Result<ToolResult>;

    /// Check if this tool invocation is allowed.
    fn check_permissions(&self, input: &Value, context: &ToolUseContext) -> PermissionResult {
        let _ = input;
        PermissionResult::Prompt {
            message: format!("Allow {}?", self.name()),
        }
    }

    /// Validate input structure before execution.
    fn validate_input(&self, _input: &Value) -> ValidationResult {
        ValidationResult::Ok
    }

    /// Whether this tool can run concurrently with other tool calls.
    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        false
    }

    /// Whether this tool only reads state (no side effects).
    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    /// Whether this tool might cause destructive/irreversible changes.
    fn is_destructive(&self, _input: &Value) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_constructors() {
        let ok = ToolResult::success(serde_json::json!({"lines": 42}));
        assert!(!ok.is_error);

        let text = ToolResult::success_text("done");
        assert_eq!(text.display.as_deref(), Some("done"));

        let err = ToolResult::error("file not found");
        assert!(err.is_error);
        assert_eq!(err.display.as_deref(), Some("file not found"));
    }
}
