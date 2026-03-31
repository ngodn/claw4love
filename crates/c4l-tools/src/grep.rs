//! GrepTool: search file contents using ripgrep.
//!
//! Maps from: leak-claude-code/src/tools/GrepTool/GrepTool.ts
//! Shells out to `rg` (ripgrep), same approach as the TypeScript version.

use crate::traits::{Tool, ToolResult, ToolUseContext};
use async_trait::async_trait;
use c4l_types::{PermissionResult, ToolInputSchema};
use serde_json::Value;
use std::collections::HashMap;
use tokio::process::Command;

pub struct GrepTool;

const DEFAULT_HEAD_LIMIT: usize = 250;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> &str {
        "Search file contents using ripgrep. Supports regex, file type filtering, and context lines."
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::from([
                ("pattern".into(), serde_json::json!({"type": "string", "description": "Regex pattern to search for"})),
                ("path".into(), serde_json::json!({"type": "string", "description": "File or directory to search in"})),
                ("glob".into(), serde_json::json!({"type": "string", "description": "Glob to filter files (e.g. \"*.rs\")"})),
                ("type".into(), serde_json::json!({"type": "string", "description": "File type (e.g. \"rs\", \"py\", \"js\")"})),
                ("output_mode".into(), serde_json::json!({"type": "string", "enum": ["content", "files_with_matches", "count"], "default": "files_with_matches"})),
                ("context".into(), serde_json::json!({"type": "number", "description": "Context lines around matches"})),
                ("head_limit".into(), serde_json::json!({"type": "number", "description": "Max results (default 250)"})),
            ])),
            required: Some(vec!["pattern".into()]),
            extra: HashMap::new(),
        }
    }

    fn check_permissions(&self, _input: &Value, _context: &ToolUseContext) -> PermissionResult {
        PermissionResult::Allow { updated_input: None }
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }
    fn is_concurrency_safe(&self, _input: &Value) -> bool { true }

    async fn call(&self, input: Value, context: &ToolUseContext) -> anyhow::Result<ToolResult> {
        let pattern = input.get("pattern").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing pattern"))?;

        let search_path = input.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let output_mode = input.get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches");

        let head_limit = input.get("head_limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_HEAD_LIMIT as u64) as usize;

        // Build rg command
        let mut cmd = Command::new("rg");
        cmd.current_dir(&context.working_directory);

        match output_mode {
            "files_with_matches" => { cmd.arg("-l"); }
            "count" => { cmd.arg("-c"); }
            _ => { cmd.arg("-n"); } // content mode: show line numbers
        }

        // Context lines
        if let Some(ctx) = input.get("context").and_then(|v| v.as_u64()) {
            cmd.arg("-C").arg(ctx.to_string());
        }

        // File type filter
        if let Some(file_type) = input.get("type").and_then(|v| v.as_str()) {
            cmd.arg("--type").arg(file_type);
        }

        // Glob filter
        if let Some(glob_pat) = input.get("glob").and_then(|v| v.as_str()) {
            cmd.arg("--glob").arg(glob_pat);
        }

        // Max count to avoid huge output
        cmd.arg("--max-count").arg("1000");

        cmd.arg(pattern);
        cmd.arg(search_path);

        let output = cmd.output().await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !output.status.success() && stdout.is_empty() {
                    if stderr.contains("No files were searched") || output.status.code() == Some(1) {
                        return Ok(ToolResult::success_text("No matches found"));
                    }
                    return Ok(ToolResult::error(format!("rg error: {stderr}")));
                }

                // Apply head limit
                let lines: Vec<&str> = stdout.lines().collect();
                let truncated = lines.len() > head_limit;
                let selected: Vec<&str> = lines.into_iter().take(head_limit).collect();
                let mut result = selected.join("\n");

                if truncated {
                    result.push_str(&format!("\n\n(truncated at {head_limit} results)"));
                }

                if result.is_empty() {
                    Ok(ToolResult::success_text("No matches found"))
                } else {
                    Ok(ToolResult::success_text(result))
                }
            }
            Err(e) => {
                Ok(ToolResult::error(format!(
                    "Failed to run ripgrep. Is 'rg' installed? Error: {e}"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context(dir: &std::path::Path) -> ToolUseContext {
        ToolUseContext {
            working_directory: dir.to_path_buf(),
            permission_context: Default::default(),
            verbose: false,
        }
    }

    #[tokio::test]
    async fn grep_finds_pattern() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.rs"), "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let tool = GrepTool;
        let result = tool.call(
            serde_json::json!({"pattern": "println", "output_mode": "content"}),
            &test_context(dir.path()),
        ).await.unwrap();

        assert!(!result.is_error);
        assert!(result.data.as_str().unwrap().contains("println"));
    }

    #[tokio::test]
    async fn grep_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();

        let tool = GrepTool;
        let result = tool.call(
            serde_json::json!({"pattern": "nonexistent_string_xyz"}),
            &test_context(dir.path()),
        ).await.unwrap();

        assert!(result.data.as_str().unwrap().contains("No matches"));
    }

    #[tokio::test]
    async fn grep_files_with_matches_mode() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn foo() {}").unwrap();
        std::fs::write(dir.path().join("b.rs"), "fn bar() {}").unwrap();
        std::fs::write(dir.path().join("c.txt"), "no functions here").unwrap();

        let tool = GrepTool;
        let result = tool.call(
            serde_json::json!({"pattern": "fn ", "output_mode": "files_with_matches"}),
            &test_context(dir.path()),
        ).await.unwrap();

        let text = result.data.as_str().unwrap();
        assert!(text.contains("a.rs"));
        assert!(text.contains("b.rs"));
        assert!(!text.contains("c.txt"));
    }

    #[test]
    fn is_read_only() {
        assert!(GrepTool.is_read_only(&Value::Null));
        assert!(GrepTool.is_concurrency_safe(&Value::Null));
    }
}
