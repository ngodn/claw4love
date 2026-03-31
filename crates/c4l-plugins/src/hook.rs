//! Hook system: event-driven automations (pre/post tool use, session lifecycle).
//!
//! Maps from: ECC hooks.json + Superpowers hooks/ + TypeScript hook system
//! Hook scripts receive JSON on stdin, return JSON on stdout.
//! PreToolUse hooks: exit code 2 = block tool execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, warn};

/// Hook event types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    SessionStart,
    SessionEnd,
    Stop,
    PreCompact,
}

/// A hook definition loaded from hooks.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    pub event: HookEvent,
    /// Tool name pattern to match (e.g., "Bash", "Edit|Write").
    /// None means match all tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// Shell command to execute.
    pub command: String,
    /// Run asynchronously (don't wait for result).
    #[serde(default)]
    pub r#async: bool,
}

/// Input passed to hook scripts as JSON on stdin.
#[derive(Debug, Clone, Serialize)]
pub struct HookInput {
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<serde_json::Value>,
}

/// Result from executing a hook.
#[derive(Debug, Clone)]
pub struct HookResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    /// Parsed additional context from hook output.
    pub additional_context: Option<String>,
}

impl HookResult {
    /// Whether this hook blocked the tool execution (exit code 2).
    pub fn is_blocked(&self) -> bool {
        self.exit_code == 2
    }
}

/// Load hooks from a hooks.json file.
///
/// Expected format (from ECC):
/// ```json
/// {
///   "hooks": {
///     "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "..." }] }]
///   }
/// }
/// ```
///
/// We also support a flat format:
/// ```json
/// [{ "event": "PreToolUse", "matcher": "Bash", "command": "..." }]
/// ```
pub fn load_hooks(paths: &[PathBuf]) -> Vec<HookDef> {
    let mut hooks = Vec::new();

    for path in paths {
        if !path.exists() {
            continue;
        }

        match load_hooks_from_file(path) {
            Ok(mut loaded) => hooks.append(&mut loaded),
            Err(e) => warn!(?path, %e, "failed to load hooks"),
        }
    }

    hooks
}

fn load_hooks_from_file(path: &Path) -> Result<Vec<HookDef>> {
    let content = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;

    // Try flat array format first
    if let Ok(hooks) = serde_json::from_value::<Vec<HookDef>>(value.clone()) {
        return Ok(hooks);
    }

    // Try ECC nested format
    let mut hooks = Vec::new();
    if let Some(hooks_obj) = value.get("hooks").and_then(|h| h.as_object()) {
        for (event_name, entries) in hooks_obj {
            let event = match event_name.as_str() {
                "PreToolUse" => HookEvent::PreToolUse,
                "PostToolUse" => HookEvent::PostToolUse,
                "SessionStart" => HookEvent::SessionStart,
                "SessionEnd" => HookEvent::SessionEnd,
                "Stop" => HookEvent::Stop,
                "PreCompact" => HookEvent::PreCompact,
                _ => continue,
            };

            if let Some(entries) = entries.as_array() {
                for entry in entries {
                    let matcher = entry.get("matcher").and_then(|m| m.as_str()).map(String::from);
                    if let Some(inner_hooks) = entry.get("hooks").and_then(|h| h.as_array()) {
                        for hook in inner_hooks {
                            if let Some(command) = hook.get("command").and_then(|c| c.as_str()) {
                                let is_async = hook.get("async").and_then(|a| a.as_bool()).unwrap_or(false);
                                hooks.push(HookDef {
                                    event: event.clone(),
                                    matcher: matcher.clone(),
                                    command: command.to_string(),
                                    r#async: is_async,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(hooks)
}

/// Execute all hooks matching an event and tool name.
///
/// For PreToolUse: if any hook exits with code 2, the tool should be blocked.
pub async fn execute_hooks(
    hooks: &[HookDef],
    event: &HookEvent,
    input: &HookInput,
) -> Vec<HookResult> {
    let matching: Vec<&HookDef> = hooks
        .iter()
        .filter(|h| {
            h.event == *event
                && h.matcher
                    .as_ref()
                    .map(|m| {
                        m.split('|')
                            .any(|pat| pat.trim() == "*" || input.tool_name.contains(pat.trim()))
                    })
                    .unwrap_or(true)
        })
        .collect();

    let mut results = Vec::new();
    let input_json = serde_json::to_string(input).unwrap_or_default();

    for hook in matching {
        debug!(command = %hook.command, "executing hook");

        let result = Command::new("sh")
            .arg("-c")
            .arg(&hook.command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                // Write input JSON to stdin
                if let Some(mut stdin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    let _ = stdin.write_all(input_json.as_bytes()).await;
                    drop(stdin);
                }

                match child.wait_with_output().await {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let exit_code = output.status.code().unwrap_or(-1);

                        // Try to parse additional context from stdout
                        let additional_context = serde_json::from_str::<serde_json::Value>(&stdout)
                            .ok()
                            .and_then(|v| {
                                v.get("additionalContext")
                                    .or_else(|| v.get("additional_context"))
                                    .and_then(|c| c.as_str())
                                    .map(String::from)
                            });

                        results.push(HookResult {
                            exit_code,
                            stdout,
                            stderr,
                            additional_context,
                        });
                    }
                    Err(e) => {
                        warn!(command = %hook.command, %e, "hook execution failed");
                        results.push(HookResult {
                            exit_code: -1,
                            stdout: String::new(),
                            stderr: e.to_string(),
                            additional_context: None,
                        });
                    }
                }
            }
            Err(e) => {
                warn!(command = %hook.command, %e, "failed to spawn hook");
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_flat_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("hooks.json");
        std::fs::write(&file, r#"[
            {"event": "PreToolUse", "matcher": "Bash", "command": "echo test", "async": false}
        ]"#).unwrap();

        let hooks = load_hooks(&[file]);
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].event, HookEvent::PreToolUse);
        assert_eq!(hooks[0].matcher.as_deref(), Some("Bash"));
    }

    #[test]
    fn load_ecc_nested_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("hooks.json");
        std::fs::write(&file, r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {"type": "command", "command": "echo pre-bash"}
                        ]
                    }
                ],
                "SessionStart": [
                    {
                        "hooks": [
                            {"type": "command", "command": "echo start", "async": true}
                        ]
                    }
                ]
            }
        }"#).unwrap();

        let hooks = load_hooks(&[file]);
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0].event, HookEvent::PreToolUse);
        assert_eq!(hooks[1].event, HookEvent::SessionStart);
        assert!(hooks[1].r#async);
    }

    #[test]
    fn missing_file_returns_empty() {
        let hooks = load_hooks(&[PathBuf::from("/nonexistent/hooks.json")]);
        assert!(hooks.is_empty());
    }

    #[tokio::test]
    async fn execute_echo_hook() {
        let hooks = vec![HookDef {
            event: HookEvent::PreToolUse,
            matcher: Some("Bash".into()),
            command: "echo ok".into(),
            r#async: false,
        }];

        let input = HookInput {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
            tool_output: None,
        };

        let results = execute_hooks(&hooks, &HookEvent::PreToolUse, &input).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].exit_code, 0);
        assert!(results[0].stdout.contains("ok"));
    }

    #[tokio::test]
    async fn hook_matcher_filters() {
        let hooks = vec![
            HookDef {
                event: HookEvent::PreToolUse,
                matcher: Some("Bash".into()),
                command: "echo bash-hook".into(),
                r#async: false,
            },
            HookDef {
                event: HookEvent::PreToolUse,
                matcher: Some("Edit".into()),
                command: "echo edit-hook".into(),
                r#async: false,
            },
        ];

        let input = HookInput {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({}),
            tool_output: None,
        };

        let results = execute_hooks(&hooks, &HookEvent::PreToolUse, &input).await;
        assert_eq!(results.len(), 1); // only Bash hook matches
        assert!(results[0].stdout.contains("bash-hook"));
    }

    #[test]
    fn hook_result_blocked() {
        let result = HookResult {
            exit_code: 2,
            stdout: String::new(),
            stderr: "blocked".into(),
            additional_context: None,
        };
        assert!(result.is_blocked());

        let ok = HookResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            additional_context: None,
        };
        assert!(!ok.is_blocked());
    }
}
