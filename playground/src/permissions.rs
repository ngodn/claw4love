//! Permission types for the tool access control system.
//!
//! Maps from: leak-claude-code/src/types/permissions.ts + src/Tool.ts ToolPermissionContext
//! Verified fields from agent research of actual TypeScript source.

use serde::{Deserialize, Serialize};

/// Permission mode — how tool access is controlled.
///
/// Verified from: leak-claude-code/src/types/permissions.ts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    Default,
    Plan,
    BypassPermissions,
    Auto,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::Default
    }
}

/// Result of a permission check on a tool invocation.
///
/// Maps from: TypeScript PermissionResult union type in Tool.ts
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Tool use is allowed, optionally with modified input.
    Allow {
        updated_input: Option<serde_json::Value>,
    },
    /// Tool use is denied with a reason.
    Deny { reason: String },
    /// User should be prompted for approval.
    Prompt { message: String },
}

/// A rule governing tool permissions.
///
/// Pattern examples from TypeScript:
/// - "Bash(git *)" — allow all git commands via Bash
/// - "FileEdit(/src/*)" — allow edits to files under /src/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionRule {
    pub tool_name: String,
    pub pattern: String,
}

impl ToolPermissionRule {
    /// Check if this rule matches a given tool name and input.
    pub fn matches(&self, tool_name: &str, input_summary: &str) -> bool {
        if self.tool_name != tool_name {
            return false;
        }
        // Simple glob-style matching for the pattern
        if self.pattern == "*" {
            return true;
        }
        input_summary.contains(&self.pattern)
    }
}

/// Context for permission checks — carried through tool execution.
///
/// Verified from: ToolPermissionContext in leak-claude-code/src/Tool.ts lines 48-62
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionContext {
    pub mode: PermissionMode,
    pub always_allow_rules: Vec<ToolPermissionRule>,
    pub always_deny_rules: Vec<ToolPermissionRule>,
    pub always_ask_rules: Vec<ToolPermissionRule>,
    #[serde(default)]
    pub is_bypass_available: bool,
    #[serde(default)]
    pub is_auto_available: bool,
}

impl Default for ToolPermissionContext {
    fn default() -> Self {
        Self {
            mode: PermissionMode::Default,
            always_allow_rules: vec![],
            always_deny_rules: vec![],
            always_ask_rules: vec![],
            is_bypass_available: false,
            is_auto_available: false,
        }
    }
}

impl ToolPermissionContext {
    /// Check a tool invocation against the permission rules.
    pub fn check(&self, tool_name: &str, input_summary: &str) -> PermissionResult {
        // Bypass mode: allow everything
        if self.mode == PermissionMode::BypassPermissions {
            return PermissionResult::Allow { updated_input: None };
        }

        // Check deny rules first
        for rule in &self.always_deny_rules {
            if rule.matches(tool_name, input_summary) {
                return PermissionResult::Deny {
                    reason: format!("Denied by rule: {}({})", rule.tool_name, rule.pattern),
                };
            }
        }

        // Check allow rules
        for rule in &self.always_allow_rules {
            if rule.matches(tool_name, input_summary) {
                return PermissionResult::Allow { updated_input: None };
            }
        }

        // Check ask rules
        for rule in &self.always_ask_rules {
            if rule.matches(tool_name, input_summary) {
                return PermissionResult::Prompt {
                    message: format!("Allow {}({})?", tool_name, input_summary),
                };
            }
        }

        // Default: prompt for approval
        PermissionResult::Prompt {
            message: format!("Allow {} with input: {}?", tool_name, input_summary),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_mode_allows_everything() {
        let ctx = ToolPermissionContext {
            mode: PermissionMode::BypassPermissions,
            ..Default::default()
        };

        match ctx.check("BashTool", "rm -rf /") {
            PermissionResult::Allow { .. } => {}
            other => panic!("expected Allow, got {:?}", other),
        }
    }

    #[test]
    fn deny_rule_blocks() {
        let ctx = ToolPermissionContext {
            always_deny_rules: vec![ToolPermissionRule {
                tool_name: "BashTool".into(),
                pattern: "rm -rf".into(),
            }],
            ..Default::default()
        };

        match ctx.check("BashTool", "rm -rf /tmp") {
            PermissionResult::Deny { reason } => {
                assert!(reason.contains("Denied"));
            }
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    #[test]
    fn allow_rule_permits() {
        let ctx = ToolPermissionContext {
            always_allow_rules: vec![ToolPermissionRule {
                tool_name: "BashTool".into(),
                pattern: "git".into(),
            }],
            ..Default::default()
        };

        match ctx.check("BashTool", "git status") {
            PermissionResult::Allow { .. } => {}
            other => panic!("expected Allow, got {:?}", other),
        }
    }

    #[test]
    fn default_mode_prompts() {
        let ctx = ToolPermissionContext::default();

        match ctx.check("BashTool", "ls -la") {
            PermissionResult::Prompt { .. } => {}
            other => panic!("expected Prompt, got {:?}", other),
        }
    }

    #[test]
    fn permission_mode_serde_roundtrip() {
        let mode = PermissionMode::BypassPermissions;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"bypassPermissions\"");

        let back: PermissionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PermissionMode::BypassPermissions);
    }
}
