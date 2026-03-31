//! Command type definitions for slash commands.
//!
//! Maps from: leak-claude-code/src/commands.ts
//! The trait itself lives in c4l-commands; these are shared data types.

use serde::{Deserialize, Serialize};

/// Command manifest entry for registration/discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandManifest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    pub description: String,
    pub command_type: CommandType,
    pub source: CommandSource,
}

/// How a command is executed.
///
/// Maps from: PromptCommand | LocalCommand | LocalJSXCommand in commands.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    /// Sends formatted prompt to LLM (e.g., /commit, /review)
    Prompt,
    /// Runs in-process, returns text (e.g., /cost, /help)
    Local,
    /// Runs in-process, renders UI (e.g., /doctor, /config)
    /// In Rust: returns ratatui widgets instead of React JSX
    LocalUi,
}

/// Where a command comes from.
///
/// Maps from: commands.ts import patterns + feature gates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandSource {
    Builtin,
    InternalOnly,
    FeatureGated(String),
    Plugin(String),
    Skill(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_manifest_roundtrip() {
        let cmd = CommandManifest {
            name: "commit".into(),
            aliases: Some(vec!["ci".into()]),
            description: "Generate a commit message".into(),
            command_type: CommandType::Prompt,
            source: CommandSource::Builtin,
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let back: CommandManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "commit");
    }
}
