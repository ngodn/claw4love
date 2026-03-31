//! FileReadTool: read files from the filesystem.
//!
//! Maps from: leak-claude-code/src/tools/FileReadTool/FileReadTool.ts

use crate::traits::{Tool, ToolResult, ToolUseContext};
use async_trait::async_trait;
use c4l_types::{PermissionResult, ToolInputSchema, ValidationResult};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub struct FileReadTool;

const DEFAULT_LINE_LIMIT: usize = 2000;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "Read"
    }

    fn description(&self) -> &str {
        "Reads a file from the local filesystem. Results are returned with line numbers."
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::from([
                (
                    "file_path".into(),
                    serde_json::json!({"type": "string", "description": "Absolute path to the file to read"}),
                ),
                (
                    "offset".into(),
                    serde_json::json!({"type": "number", "description": "Line number to start reading from (0-based)"}),
                ),
                (
                    "limit".into(),
                    serde_json::json!({"type": "number", "description": "Number of lines to read"}),
                ),
            ])),
            required: Some(vec!["file_path".into()]),
            extra: HashMap::new(),
        }
    }

    fn check_permissions(&self, _input: &Value, _context: &ToolUseContext) -> PermissionResult {
        PermissionResult::Allow { updated_input: None }
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    fn is_concurrency_safe(&self, _input: &Value) -> bool {
        true
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("file_path").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Err {
                message: "file_path is required".into(),
                error_code: 400,
            },
        }
    }

    async fn call(&self, input: Value, _context: &ToolUseContext) -> anyhow::Result<ToolResult> {
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing file_path"))?;

        let offset = input.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_LINE_LIMIT as u64) as usize;

        let path = Path::new(file_path);

        if !path.exists() {
            return Ok(ToolResult::error(format!("File not found: {file_path}")));
        }

        if path.is_dir() {
            return Ok(ToolResult::error(format!(
                "{file_path} is a directory, not a file. Use Bash with ls to list directory contents."
            )));
        }

        // Read the file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::error(format!("Failed to read {file_path}: {e}")));
            }
        };

        // Apply offset and limit, format with line numbers (cat -n style)
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let start = offset.min(total_lines);
        let end = (start + limit).min(total_lines);
        let selected = &lines[start..end];

        let mut output = String::new();
        for (i, line) in selected.iter().enumerate() {
            let line_num = start + i + 1; // 1-based
            output.push_str(&format!("{line_num}\t{line}\n"));
        }

        if end < total_lines {
            output.push_str(&format!(
                "\n... ({} more lines, use offset/limit to read more)\n",
                total_lines - end
            ));
        }

        Ok(ToolResult::success_text(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context() -> ToolUseContext {
        ToolUseContext {
            working_directory: PathBuf::from("/tmp"),
            permission_context: Default::default(),
            verbose: false,
        }
    }

    #[tokio::test]
    async fn read_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "line one\nline two\nline three\n").unwrap();

        let tool = FileReadTool;
        let ctx = test_context();
        let result = tool
            .call(
                serde_json::json!({"file_path": file.to_str().unwrap()}),
                &ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = result.data.as_str().unwrap();
        assert!(text.contains("1\tline one"));
        assert!(text.contains("2\tline two"));
        assert!(text.contains("3\tline three"));
    }

    #[tokio::test]
    async fn read_with_offset_limit() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\nd\ne\n").unwrap();

        let tool = FileReadTool;
        let ctx = test_context();
        let result = tool
            .call(
                serde_json::json!({"file_path": file.to_str().unwrap(), "offset": 1, "limit": 2}),
                &ctx,
            )
            .await
            .unwrap();

        let text = result.data.as_str().unwrap();
        assert!(text.contains("2\tb"));
        assert!(text.contains("3\tc"));
        assert!(!text.contains("1\ta"));
    }

    #[tokio::test]
    async fn read_missing_file() {
        let tool = FileReadTool;
        let ctx = test_context();
        let result = tool
            .call(
                serde_json::json!({"file_path": "/nonexistent/path/file.txt"}),
                &ctx,
            )
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[test]
    fn is_read_only() {
        assert!(FileReadTool.is_read_only(&Value::Null));
        assert!(FileReadTool.is_concurrency_safe(&Value::Null));
    }
}
