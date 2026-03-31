//! BashTool: execute shell commands.
//!
//! Maps from: leak-claude-code/src/tools/BashTool/BashTool.ts

use crate::traits::{Tool, ToolResult, ToolUseContext};
use async_trait::async_trait;
use c4l_types::{PermissionResult, ToolInputSchema, ValidationResult};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::process::Command;
use tracing::debug;

pub struct BashTool;

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const MAX_TIMEOUT_SECS: u64 = 600;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn description(&self) -> &str {
        "Executes a given bash command and returns its output."
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::from([
                (
                    "command".into(),
                    serde_json::json!({"type": "string", "description": "The command to execute"}),
                ),
                (
                    "description".into(),
                    serde_json::json!({"type": "string", "description": "Short description of what the command does"}),
                ),
                (
                    "timeout".into(),
                    serde_json::json!({"type": "number", "description": "Timeout in milliseconds (max 600000)"}),
                ),
            ])),
            required: Some(vec!["command".into()]),
            extra: HashMap::new(),
        }
    }

    fn validate_input(&self, input: &Value) -> ValidationResult {
        match input.get("command").and_then(|v| v.as_str()) {
            Some(cmd) if !cmd.trim().is_empty() => ValidationResult::Ok,
            _ => ValidationResult::Err {
                message: "command is required and must be a non-empty string".into(),
                error_code: 400,
            },
        }
    }

    fn check_permissions(&self, input: &Value, context: &ToolUseContext) -> PermissionResult {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        context.permission_context.check("Bash", command)
    }

    async fn call(&self, input: Value, context: &ToolUseContext) -> anyhow::Result<ToolResult> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing command"))?;

        let timeout_ms = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS * 1000);
        let timeout_secs = (timeout_ms / 1000).min(MAX_TIMEOUT_SECS);

        debug!(command, timeout_secs, "executing bash command");

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());

        let output = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            Command::new(&shell)
                .arg("-c")
                .arg(command)
                .current_dir(&context.working_directory)
                .output(),
        )
        .await;

        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                let mut result = String::new();
                if !stdout.is_empty() {
                    result.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&stderr);
                }

                if exit_code != 0 {
                    result.push_str(&format!("\n\nExit code: {exit_code}"));
                }

                Ok(ToolResult {
                    data: Value::String(result.clone()),
                    is_error: exit_code != 0,
                    display: Some(result),
                })
            }
            Ok(Err(e)) => Ok(ToolResult::error(format!("Failed to execute command: {e}"))),
            Err(_) => Ok(ToolResult::error(format!(
                "Command timed out after {timeout_secs}s"
            ))),
        }
    }

    fn is_destructive(&self, input: &Value) -> bool {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // Simple heuristic for destructive commands
        cmd.contains("rm -rf")
            || cmd.contains("git reset --hard")
            || cmd.contains("git push --force")
            || cmd.contains("drop table")
            || cmd.contains("DROP TABLE")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context() -> ToolUseContext {
        ToolUseContext {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            permission_context: c4l_types::ToolPermissionContext {
                mode: c4l_types::PermissionMode::BypassPermissions,
                ..Default::default()
            },
            verbose: false,
        }
    }

    #[tokio::test]
    async fn echo_command() {
        let tool = BashTool;
        let ctx = test_context();
        let result = tool
            .call(serde_json::json!({"command": "echo hello"}), &ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.data.as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn failing_command() {
        let tool = BashTool;
        let ctx = test_context();
        let result = tool
            .call(serde_json::json!({"command": "false"}), &ctx)
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[test]
    fn validates_empty_command() {
        let tool = BashTool;
        let result = tool.validate_input(&serde_json::json!({"command": ""}));
        assert!(matches!(result, ValidationResult::Err { .. }));
    }

    #[test]
    fn detects_destructive() {
        let tool = BashTool;
        assert!(tool.is_destructive(&serde_json::json!({"command": "rm -rf /"})));
        assert!(!tool.is_destructive(&serde_json::json!({"command": "ls -la"})));
    }
}
