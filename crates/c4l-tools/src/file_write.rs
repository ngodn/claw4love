//! FileWriteTool: create or overwrite files.
//!
//! Maps from: leak-claude-code/src/tools/FileWriteTool/FileWriteTool.ts

use crate::traits::{Tool, ToolResult, ToolUseContext};
use async_trait::async_trait;
use c4l_types::{PermissionResult, ToolInputSchema, ValidationResult};
use serde_json::Value;
use std::collections::HashMap;

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn description(&self) -> &str {
        "Writes a file to the local filesystem. Creates parent directories if needed."
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::from([
                ("file_path".into(), serde_json::json!({"type": "string", "description": "Absolute path to the file to write"})),
                ("content".into(), serde_json::json!({"type": "string", "description": "The content to write"})),
            ])),
            required: Some(vec!["file_path".into(), "content".into()]),
            extra: HashMap::new(),
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        if input.get("file_path").and_then(|v| v.as_str()).is_none() {
            return ValidationResult::Err { message: "file_path is required".into(), error_code: 400 };
        }
        if input.get("content").and_then(|v| v.as_str()).is_none() {
            return ValidationResult::Err { message: "content is required".into(), error_code: 400 };
        }
        ValidationResult::Ok
    }

    fn check_permissions(&self, input: &Value, context: &ToolUseContext) -> PermissionResult {
        let file_path = input.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
        context.permission_context.check("Write", file_path)
    }

    fn is_destructive(&self, input: &Value) -> bool {
        // Destructive if overwriting an existing file
        input.get("file_path")
            .and_then(|v| v.as_str())
            .map(|p| std::path::Path::new(p).exists())
            .unwrap_or(false)
    }

    async fn call(&self, input: Value, _context: &ToolUseContext) -> anyhow::Result<ToolResult> {
        let file_path = input.get("file_path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing file_path"))?;
        let content = input.get("content").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing content"))?;

        let path = std::path::Path::new(file_path);

        // Create parent directories
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(path, content)?;

        let action = if path.exists() { "Updated" } else { "Created" };
        Ok(ToolResult::success_text(format!("{action} {file_path}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context() -> ToolUseContext {
        ToolUseContext {
            working_directory: PathBuf::from("/tmp"),
            permission_context: c4l_types::ToolPermissionContext {
                mode: c4l_types::PermissionMode::BypassPermissions,
                ..Default::default()
            },
            verbose: false,
        }
    }

    #[tokio::test]
    async fn write_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("new.txt");

        let tool = FileWriteTool;
        let result = tool.call(
            serde_json::json!({"file_path": file.to_str().unwrap(), "content": "hello world"}),
            &test_context(),
        ).await.unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello world");
    }

    #[tokio::test]
    async fn write_creates_parents() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("deep").join("nested").join("file.txt");

        let tool = FileWriteTool;
        let result = tool.call(
            serde_json::json!({"file_path": file.to_str().unwrap(), "content": "nested content"}),
            &test_context(),
        ).await.unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "nested content");
    }

    #[tokio::test]
    async fn write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("existing.txt");
        std::fs::write(&file, "old content").unwrap();

        let tool = FileWriteTool;
        let result = tool.call(
            serde_json::json!({"file_path": file.to_str().unwrap(), "content": "new content"}),
            &test_context(),
        ).await.unwrap();

        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "new content");
    }
}
