//! GlobTool: find files matching glob patterns.
//!
//! Maps from: leak-claude-code/src/tools/GlobTool/GlobTool.ts

use crate::traits::{Tool, ToolResult, ToolUseContext};
use async_trait::async_trait;
use c4l_types::{PermissionResult, ToolInputSchema, ValidationResult};
use ignore::WalkBuilder;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct GlobTool;

const DEFAULT_MAX_RESULTS: usize = 1000;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        "Fast file pattern matching tool. Supports glob patterns like \"**/*.rs\". Returns matching file paths sorted by modification time."
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::from([
                ("pattern".into(), serde_json::json!({"type": "string", "description": "Glob pattern to match (e.g. \"**/*.rs\", \"src/**/*.ts\")"})),
                ("path".into(), serde_json::json!({"type": "string", "description": "Directory to search in (defaults to working directory)"})),
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

        let search_dir = input.get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| context.working_directory.clone());

        let glob = globset::GlobBuilder::new(pattern)
            .literal_separator(false)
            .build()
            .map_err(|e| anyhow::anyhow!("invalid glob pattern: {e}"))?
            .compile_matcher();

        let mut matches: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        // Walk directory respecting .gitignore (like RTK's ignore crate usage)
        for entry in WalkBuilder::new(&search_dir)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .build()
            .flatten()
        {
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }

            let path = entry.path();
            let relative = path.strip_prefix(&search_dir).unwrap_or(path);

            if glob.is_match(relative) || glob.is_match(path) {
                let mtime = entry.metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                matches.push((path.to_path_buf(), mtime));

                if matches.len() >= DEFAULT_MAX_RESULTS {
                    break;
                }
            }
        }

        // Sort by modification time (most recent first)
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        let output: Vec<String> = matches.iter().map(|(p, _)| p.display().to_string()).collect();
        let count = output.len();

        if output.is_empty() {
            Ok(ToolResult::success_text("No files matched the pattern."))
        } else {
            let text = output.join("\n");
            let suffix = if count >= DEFAULT_MAX_RESULTS {
                format!("\n\n(showing first {DEFAULT_MAX_RESULTS} results)")
            } else {
                String::new()
            };
            Ok(ToolResult::success_text(format!("{text}{suffix}")))
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
    async fn glob_finds_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.rs"), "").unwrap();
        std::fs::write(dir.path().join("bar.rs"), "").unwrap();
        std::fs::write(dir.path().join("baz.txt"), "").unwrap();

        let tool = GlobTool;
        let result = tool.call(
            serde_json::json!({"pattern": "*.rs"}),
            &test_context(dir.path()),
        ).await.unwrap();

        let text = result.data.as_str().unwrap();
        assert!(text.contains("foo.rs"));
        assert!(text.contains("bar.rs"));
        assert!(!text.contains("baz.txt"));
    }

    #[tokio::test]
    async fn glob_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.txt"), "").unwrap();

        let tool = GlobTool;
        let result = tool.call(
            serde_json::json!({"pattern": "*.rs"}),
            &test_context(dir.path()),
        ).await.unwrap();

        assert!(result.data.as_str().unwrap().contains("No files matched"));
    }

    #[tokio::test]
    async fn glob_recursive() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("src");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("main.rs"), "").unwrap();
        std::fs::write(dir.path().join("lib.rs"), "").unwrap();

        let tool = GlobTool;
        let result = tool.call(
            serde_json::json!({"pattern": "**/*.rs"}),
            &test_context(dir.path()),
        ).await.unwrap();

        let text = result.data.as_str().unwrap();
        assert!(text.contains("main.rs"));
        assert!(text.contains("lib.rs"));
    }
}
