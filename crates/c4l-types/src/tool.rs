//! Tool type definitions.
//!
//! Maps from: leak-claude-code/src/Tool.ts (795 lines)
//! The trait itself lives in c4l-tools; these are the shared data types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool input JSON schema — sent to the Anthropic API.
///
/// Verified from: ToolInputJSONSchema in Tool.ts lines 1-8
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // always "object"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Validation result for tool input.
///
/// Maps from: ValidationResult in Tool.ts
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Ok,
    Err { message: String, error_code: i32 },
}

/// Tool manifest entry for registration/discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_hint: Option<String>,
    pub source: ToolSource,
}

/// Where a tool comes from — for feature gating and lazy loading.
///
/// Maps from: tools.ts registration pattern (base vs conditional vs lazy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    /// Always available (BashTool, FileReadTool, etc.)
    Base,
    /// Available when a Cargo feature is enabled
    Conditional(String),
    /// Loaded on demand (TeamCreateTool, etc.)
    Lazy,
    /// Provided by an MCP server
    Mcp(String),
}

/// The result of executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Structured output data
    pub data: serde_json::Value,
    /// Whether this result represents an error
    #[serde(default)]
    pub is_error: bool,
    /// Human-readable summary for display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_input_schema_roundtrip() {
        let schema = ToolInputSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::from([(
                "command".into(),
                serde_json::json!({"type": "string", "description": "The command to execute"}),
            )])),
            required: Some(vec!["command".into()]),
            extra: HashMap::new(),
        };

        let json = serde_json::to_string(&schema).unwrap();
        let back: ToolInputSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_type, "object");
        assert!(back.properties.unwrap().contains_key("command"));
    }

    #[test]
    fn tool_result_roundtrip() {
        let result = ToolResult {
            data: serde_json::json!({"output": "hello world"}),
            is_error: false,
            display: Some("Command executed successfully".into()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let back: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(!back.is_error);
    }
}
