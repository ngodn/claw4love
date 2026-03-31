//! Tool registry: holds all registered tools and provides lookup/dispatch.

use crate::traits::{Tool, ToolResult, ToolUseContext};
use c4l_api::ApiToolDef;
use serde_json::Value;

/// Registry of available tools.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.tools.push(Box::new(tool));
    }

    /// Find a tool by name or alias.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name || t.aliases().contains(&name))
            .map(|t| t.as_ref())
    }

    pub fn all(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Build API tool definitions for the messages request.
    pub fn api_tool_defs(&self) -> Vec<ApiToolDef> {
        self.tools
            .iter()
            .map(|t| {
                let schema = t.input_schema();
                ApiToolDef {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    input_schema: serde_json::to_value(&schema).unwrap_or_default(),
                }
            })
            .collect()
    }

    /// Execute a tool by name.
    pub async fn execute(
        &self,
        name: &str,
        input: Value,
        context: &ToolUseContext,
    ) -> anyhow::Result<ToolResult> {
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("tool not found: {name}"))?;
        tool.call(input, context).await
    }

    /// Register all default tools.
    pub fn register_defaults(&mut self) {
        self.register(crate::bash::BashTool);
        self.register(crate::file_read::FileReadTool);
        self.register(crate::file_edit::FileEditTool);
        self.register(crate::file_write::FileWriteTool);
        self.register(crate::glob::GlobTool);
        self.register(crate::grep::GrepTool);
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register_defaults();
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_tools() {
        let reg = ToolRegistry::default();
        assert_eq!(reg.len(), 6);
        assert!(reg.get("Bash").is_some());
        assert!(reg.get("Read").is_some());
        assert!(reg.get("Edit").is_some());
        assert!(reg.get("Write").is_some());
        assert!(reg.get("Glob").is_some());
        assert!(reg.get("Grep").is_some());
    }

    #[test]
    fn api_tool_defs_generated() {
        let reg = ToolRegistry::default();
        let defs = reg.api_tool_defs();
        assert_eq!(defs.len(), 6);
        assert!(defs.iter().any(|d| d.name == "Bash"));
    }

    #[test]
    fn lookup_missing_tool() {
        let reg = ToolRegistry::new();
        assert!(reg.get("NonExistent").is_none());
    }
}
