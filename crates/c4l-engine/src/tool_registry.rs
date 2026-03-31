//! Tool registry: holds available tools and dispatches execution.
//!
//! The actual Tool trait and implementations live in c4l-tools (Phase 2).
//! This module provides the registry interface that the engine uses.

use anyhow::Result;
use c4l_api::ApiToolDef;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Function type for tool execution.
///
/// In Phase 2, this will be replaced by the Tool trait from c4l-tools.
/// For now, we use a simple async function pointer so the engine can be tested.
pub type ToolExecuteFn =
    Box<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<ToolExecResult>> + Send>> + Send + Sync>;

/// Result of executing a tool.
#[derive(Debug, Clone)]
pub struct ToolExecResult {
    pub content: Value,
    pub is_error: bool,
}

/// A registered tool with its metadata and executor.
pub struct RegisteredTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub execute: ToolExecuteFn,
}

/// Registry of available tools.
///
/// The engine queries this to build API tool definitions and execute tool calls.
pub struct ToolRegistry {
    tools: Vec<RegisteredTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: RegisteredTool) {
        self.tools.push(tool);
    }

    /// Find a tool by name.
    pub fn get(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Build API tool definitions for the messages request.
    pub fn api_tool_defs(&self) -> Vec<ApiToolDef> {
        self.tools
            .iter()
            .map(|t| ApiToolDef {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect()
    }

    /// Execute a tool by name.
    pub async fn execute(&self, name: &str, input: Value) -> Result<ToolExecResult> {
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("tool not found: {name}"))?;
        (tool.execute)(input).await
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_echo_tool() -> RegisteredTool {
        RegisteredTool {
            name: "EchoTool".into(),
            description: "Echoes input back".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                }
            }),
            execute: Box::new(|input| {
                Box::pin(async move {
                    Ok(ToolExecResult {
                        content: input,
                        is_error: false,
                    })
                })
            }),
        }
    }

    #[test]
    fn register_and_find() {
        let mut registry = ToolRegistry::new();
        registry.register(make_echo_tool());

        assert_eq!(registry.len(), 1);
        assert!(registry.get("EchoTool").is_some());
        assert!(registry.get("NonExistent").is_none());
    }

    #[test]
    fn api_tool_defs() {
        let mut registry = ToolRegistry::new();
        registry.register(make_echo_tool());

        let defs = registry.api_tool_defs();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "EchoTool");
    }

    #[tokio::test]
    async fn execute_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(make_echo_tool());

        let result = registry
            .execute("EchoTool", serde_json::json!({"text": "hello"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content["text"], "hello");
    }

    #[tokio::test]
    async fn execute_missing_tool_errors() {
        let registry = ToolRegistry::new();
        let result = registry
            .execute("Missing", serde_json::json!({}))
            .await;

        assert!(result.is_err());
    }
}
