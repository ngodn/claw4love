//! FileEditTool: perform string replacement edits on files.
//!
//! Maps from: leak-claude-code/src/tools/FileEditTool/FileEditTool.ts

use crate::traits::{Tool, ToolResult, ToolUseContext};
use async_trait::async_trait;
use c4l_types::{PermissionResult, ToolInputSchema, ValidationResult};
use serde_json::Value;
use std::collections::HashMap;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        "Performs exact string replacements in files."
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::from([
                ("file_path".into(), serde_json::json!({"type": "string", "description": "Absolute path to the file to modify"})),
                ("old_string".into(), serde_json::json!({"type": "string", "description": "The text to replace"})),
                ("new_string".into(), serde_json::json!({"type": "string", "description": "The replacement text"})),
                ("replace_all".into(), serde_json::json!({"type": "boolean", "description": "Replace all occurrences (default false)", "default": false})),
            ])),
            required: Some(vec!["file_path".into(), "old_string".into(), "new_string".into()]),
            extra: HashMap::new(),
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        let file_path = input.get("file_path").and_then(|v| v.as_str());
        let old_string = input.get("old_string").and_then(|v| v.as_str());
        let new_string = input.get("new_string").and_then(|v| v.as_str());

        if file_path.is_none() || old_string.is_none() || new_string.is_none() {
            return ValidationResult::Err {
                message: "file_path, old_string, and new_string are required".into(),
                error_code: 400,
            };
        }

        if old_string == new_string {
            return ValidationResult::Err {
                message: "old_string and new_string must be different".into(),
                error_code: 400,
            };
        }

        ValidationResult::Ok
    }

    fn check_permissions(&self, input: &Value, context: &ToolUseContext) -> PermissionResult {
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        context.permission_context.check("Edit", file_path)
    }

    async fn call(&self, input: Value, _context: &ToolUseContext) -> anyhow::Result<ToolResult> {
        let file_path = input.get("file_path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing file_path"))?;
        let old_string = input.get("old_string").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing old_string"))?;
        let new_string = input.get("new_string").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing new_string"))?;
        let replace_all = input.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);

        let path = std::path::Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(format!("File not found: {file_path}")));
        }

        let content = std::fs::read_to_string(path)?;

        // Check occurrences
        let count = content.matches(old_string).count();

        if count == 0 {
            return Ok(ToolResult::error(format!(
                "old_string not found in {file_path}. Make sure it matches exactly."
            )));
        }

        if count > 1 && !replace_all {
            return Ok(ToolResult::error(format!(
                "old_string found {count} times in {file_path}. Use replace_all: true to replace all, or provide more context to make it unique."
            )));
        }

        // Perform replacement
        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        std::fs::write(path, &new_content)?;

        let msg = if replace_all && count > 1 {
            format!("Replaced {count} occurrences in {file_path}")
        } else {
            format!("Updated {file_path}")
        };

        Ok(ToolResult::success_text(msg))
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
    async fn edit_unique_string() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.rs");
        std::fs::write(&file, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let tool = FileEditTool;
        let result = tool.call(
            serde_json::json!({
                "file_path": file.to_str().unwrap(),
                "old_string": "hello",
                "new_string": "world"
            }),
            &test_context(),
        ).await.unwrap();

        assert!(!result.is_error);
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains("world"));
        assert!(!content.contains("hello"));
    }

    #[tokio::test]
    async fn edit_non_unique_fails() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "aaa\nbbb\naaa\n").unwrap();

        let tool = FileEditTool;
        let result = tool.call(
            serde_json::json!({
                "file_path": file.to_str().unwrap(),
                "old_string": "aaa",
                "new_string": "ccc"
            }),
            &test_context(),
        ).await.unwrap();

        assert!(result.is_error);
        assert!(result.display.unwrap().contains("2 times"));
    }

    #[tokio::test]
    async fn edit_replace_all() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "aaa\nbbb\naaa\n").unwrap();

        let tool = FileEditTool;
        let result = tool.call(
            serde_json::json!({
                "file_path": file.to_str().unwrap(),
                "old_string": "aaa",
                "new_string": "ccc",
                "replace_all": true
            }),
            &test_context(),
        ).await.unwrap();

        assert!(!result.is_error);
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "ccc\nbbb\nccc\n");
    }

    #[tokio::test]
    async fn edit_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello world").unwrap();

        let tool = FileEditTool;
        let result = tool.call(
            serde_json::json!({
                "file_path": file.to_str().unwrap(),
                "old_string": "xyz",
                "new_string": "abc"
            }),
            &test_context(),
        ).await.unwrap();

        assert!(result.is_error);
    }
}
