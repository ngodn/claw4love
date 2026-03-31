//! Configuration loading and defaults.
//!
//! Maps from: leak-claude-code/.env.example + src/utils/config.ts
//! Pattern from: RTK's core/config.rs (TOML + env var layering)
//!
//! Load order:
//! 1. Built-in defaults
//! 2. User global: ~/.config/claw4love/config.toml
//! 3. Project local: .claw4love/config.toml
//! 4. Environment variables (ANTHROPIC_API_KEY, etc.)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct C4lConfig {
    pub auth: AuthConfig,
    pub model: ModelConfig,
    pub shell: ShellConfig,
    pub display: DisplayConfig,
    pub tracking: TrackingConfig,
}

impl Default for C4lConfig {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            model: ModelConfig::default(),
            shell: ShellConfig::default(),
            display: DisplayConfig::default(),
            tracking: TrackingConfig::default(),
        }
    }
}

/// Authentication configuration.
///
/// Maps from: ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL
/// from leak-claude-code/.env.example
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AuthConfig {
    pub api_key: Option<String>,
    pub auth_token: Option<String>,
    pub base_url: Option<String>,
    #[serde(default)]
    pub use_bedrock: bool,
    #[serde(default)]
    pub use_vertex: bool,
}

/// Model selection configuration.
///
/// Maps from: ANTHROPIC_MODEL, ANTHROPIC_SMALL_FAST_MODEL
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelConfig {
    pub default_model: String,
    pub fast_model: Option<String>,
    pub subagent_model: Option<String>,
    pub max_output_tokens: Option<u32>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default_model: "claude-sonnet-4-6".into(),
            fast_model: Some("claude-haiku-4-5".into()),
            subagent_model: None,
            max_output_tokens: None,
        }
    }
}

/// Shell execution configuration.
///
/// Maps from: CLAUDE_CODE_SHELL, CLAUDE_CODE_SHELL_PREFIX, CLAUDE_CODE_TMPDIR
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ShellConfig {
    pub shell: Option<String>,
    pub shell_prefix: Option<String>,
    pub tmpdir: Option<PathBuf>,
}

/// Display/UI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub color: bool,
    pub theme: String,
    pub verbose: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            color: true,
            theme: "dark".into(),
            verbose: false,
        }
    }
}

/// Token tracking configuration.
///
/// Pattern from: RTK's TrackingConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackingConfig {
    pub enabled: bool,
    pub database_path: Option<PathBuf>,
    pub history_days: u32,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            database_path: None,
            history_days: 90,
        }
    }
}

impl C4lConfig {
    /// Load configuration with layered precedence.
    ///
    /// 1. Defaults
    /// 2. User global config file
    /// 3. Project local config file
    /// 4. Environment variables
    pub fn load(project_dir: Option<&Path>) -> anyhow::Result<Self> {
        let mut config = Self::default();

        // Layer 2: User global
        if let Some(global_path) = Self::global_config_path() {
            if global_path.exists() {
                let content = std::fs::read_to_string(&global_path)?;
                let file_config: C4lConfig = toml::from_str(&content)?;
                config.merge(file_config);
            }
        }

        // Layer 3: Project local
        if let Some(dir) = project_dir {
            let local_path = dir.join(".claw4love").join("config.toml");
            if local_path.exists() {
                let content = std::fs::read_to_string(&local_path)?;
                let file_config: C4lConfig = toml::from_str(&content)?;
                config.merge(file_config);
            }
        }

        // Layer 4: Environment variables
        config.apply_env();

        Ok(config)
    }

    /// Path to the global config file.
    fn global_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("claw4love").join("config.toml"))
    }

    /// Merge another config into this one (non-None values override).
    fn merge(&mut self, other: C4lConfig) {
        if other.auth.api_key.is_some() {
            self.auth.api_key = other.auth.api_key;
        }
        if other.auth.auth_token.is_some() {
            self.auth.auth_token = other.auth.auth_token;
        }
        if other.auth.base_url.is_some() {
            self.auth.base_url = other.auth.base_url;
        }
        if other.auth.use_bedrock {
            self.auth.use_bedrock = true;
        }
        if other.auth.use_vertex {
            self.auth.use_vertex = true;
        }
        if other.model.default_model != "claude-sonnet-4-6" {
            self.model.default_model = other.model.default_model;
        }
        if other.model.fast_model.is_some() {
            self.model.fast_model = other.model.fast_model;
        }
        if other.model.subagent_model.is_some() {
            self.model.subagent_model = other.model.subagent_model;
        }
        if other.model.max_output_tokens.is_some() {
            self.model.max_output_tokens = other.model.max_output_tokens;
        }
        if other.shell.shell.is_some() {
            self.shell.shell = other.shell.shell;
        }
        if other.tracking.database_path.is_some() {
            self.tracking.database_path = other.tracking.database_path;
        }
    }

    /// Apply environment variable overrides.
    fn apply_env(&mut self) {
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            self.auth.api_key = Some(key);
        }
        if let Ok(token) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
            self.auth.auth_token = Some(token);
        }
        if let Ok(url) = std::env::var("ANTHROPIC_BASE_URL") {
            self.auth.base_url = Some(url);
        }
        if let Ok(model) = std::env::var("ANTHROPIC_MODEL") {
            self.model.default_model = model;
        }
        if let Ok(fast) = std::env::var("ANTHROPIC_SMALL_FAST_MODEL") {
            self.model.fast_model = Some(fast);
        }
        if let Ok(shell) = std::env::var("CLAUDE_CODE_SHELL") {
            self.shell.shell = Some(shell);
        }
        if let Ok(tmpdir) = std::env::var("CLAUDE_CODE_TMPDIR") {
            self.shell.tmpdir = Some(PathBuf::from(tmpdir));
        }
        if std::env::var("CLAUDE_CODE_USE_BEDROCK").is_ok() {
            self.auth.use_bedrock = true;
        }
        if std::env::var("CLAUDE_CODE_USE_VERTEX").is_ok() {
            self.auth.use_vertex = true;
        }
    }

    /// Get the effective API base URL.
    pub fn api_base_url(&self) -> &str {
        self.auth
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = C4lConfig::default();
        assert_eq!(config.model.default_model, "claude-sonnet-4-6");
        assert_eq!(config.api_base_url(), "https://api.anthropic.com");
        assert!(config.display.color);
        assert!(config.tracking.enabled);
        assert_eq!(config.tracking.history_days, 90);
    }

    #[test]
    fn config_toml_roundtrip() {
        let config = C4lConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let back: C4lConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(back.model.default_model, config.model.default_model);
    }

    #[test]
    fn config_toml_partial_parse() {
        let partial = r#"
[model]
default_model = "claude-opus-4-6"

[tracking]
history_days = 30
"#;
        let config: C4lConfig = toml::from_str(partial).unwrap();
        assert_eq!(config.model.default_model, "claude-opus-4-6");
        assert_eq!(config.tracking.history_days, 30);
        // Defaults still apply for unspecified fields
        assert!(config.display.color);
    }

    #[test]
    fn merge_overrides_non_default() {
        let mut base = C4lConfig::default();
        let override_config = C4lConfig {
            auth: AuthConfig {
                api_key: Some("sk-test-123".into()),
                ..Default::default()
            },
            model: ModelConfig {
                default_model: "claude-opus-4-6".into(),
                ..Default::default()
            },
            ..Default::default()
        };

        base.merge(override_config);
        assert_eq!(base.auth.api_key.as_deref(), Some("sk-test-123"));
        assert_eq!(base.model.default_model, "claude-opus-4-6");
    }
}
