# Phase 0: Workspace Layout & Core Types

## Workspace Structure

```
claw4love/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── c4l-cli/                 # Binary crate — CLI entry point
│   ├── c4l-types/               # Core types (messages, tools, commands, permissions)
│   ├── c4l-config/              # Config loading, settings, defaults
│   ├── c4l-engine/              # QueryEngine — LLM streaming, tool-call loop
│   ├── c4l-api/                 # Anthropic API client (streaming, retry, auth)
│   ├── c4l-tools/               # Tool trait + all tool implementations
│   ├── c4l-commands/            # Command trait + slash command implementations
│   ├── c4l-state/               # AppState, session store, SQLite persistence
│   ├── c4l-tui/                 # Terminal UI (ratatui)
│   ├── c4l-mcp/                 # MCP client/server
│   ├── c4l-plugins/             # Plugin/skill/hook system
│   ├── c4l-bridge/              # IDE bridge (VS Code, JetBrains)
│   └── c4l-utils/               # Shared utilities (tokens, diff, markdown, etc.)
├── src/                          # Workspace-level integration tests
├── config/                       # Default config files
└── README.md
```

## Cargo.toml (Workspace Root)

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["claw4love"]

[workspace.dependencies]
# Shared across crates — single version source
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["preserve_order"] }
anyhow = "1"
thiserror = "2"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
rusqlite = { version = "0.32", features = ["bundled"] }
ratatui = "0.29"
crossterm = "0.28"
reqwest = { version = "0.12", features = ["json", "stream"] }
toml = "0.8"
regex = "1"
dirs = "6"
git2 = "0.20"
similar = "2"
globset = "0.4"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

## Core Types (c4l-types)

Mapped from TypeScript source with exact field correspondence.

### Messages (from src/types/message.ts)

```rust
// crates/c4l-types/src/message.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Top-level message enum — discriminated union
/// Maps from: TypeScript Message = UserMessage | AssistantMessage | SystemMessage | ProgressMessage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "system")]
    System(SystemMessage),
    #[serde(rename = "progress")]
    Progress(ProgressMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    pub message: UserMessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_meta: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_compact_summary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<MessageOrigin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageContent {
    pub role: String, // always "user"
    pub content: ContentBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlock {
    Text(String),
    Blocks(Vec<ContentBlockParam>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockParam {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    pub message: serde_json::Value, // BetaMessage from Anthropic SDK
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_api_error_message: Option<bool>,
}

/// System message with subtype discrimination
/// Maps from: SystemInformationalMessage | SystemAPIErrorMessage | ...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    pub subtype: SystemMessageSubtype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<SystemMessageLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemMessageSubtype {
    Informational,
    ApiError,
    LocalCommand,
    ToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemMessageLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageOrigin {
    Agent,
    Teammate,
    Command,
    System,
    Hook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    pub tool_use_id: String,
    pub parent_tool_use_id: String,
    pub data: serde_json::Value, // Progress payload varies by tool
}
```

### Tool Types (from src/Tool.ts)

```rust
// crates/c4l-types/src/tool.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool input JSON schema — matches TypeScript ToolInputJSONSchema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String, // always "object"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Permission check result
/// Maps from: TypeScript PermissionResult
#[derive(Debug, Clone)]
pub enum PermissionResult {
    Allow { updated_input: Option<serde_json::Value> },
    Deny { reason: String },
    Prompt { prompt: String },
}

/// Validation result
/// Maps from: TypeScript ValidationResult
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Ok,
    Err { message: String, error_code: i32 },
}

/// Tool metadata for registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_hint: Option<String>,
    pub source: ToolSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    Base,
    Conditional(String), // feature flag name
    Lazy,
    Mcp(String),         // server name
}
```

### Permission Types (from src/types/permissions.ts, src/Tool.ts)

```rust
// crates/c4l-types/src/permissions.rs

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    Default,
    Plan,
    BypassPermissions,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionContext {
    pub mode: PermissionMode,
    pub always_allow_rules: Vec<ToolPermissionRule>,
    pub always_deny_rules: Vec<ToolPermissionRule>,
    pub always_ask_rules: Vec<ToolPermissionRule>,
    pub is_bypass_available: bool,
    pub is_auto_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionRule {
    pub tool_name: String,
    pub pattern: String,
}
```

### Command Types (from src/commands.ts)

```rust
// crates/c4l-types/src/command.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandManifest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    pub description: String,
    pub command_type: CommandType,
    pub source: CommandSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    Prompt,     // sends formatted prompt to LLM
    Local,      // in-process, returns text
    LocalJsx,   // in-process, returns UI (→ ratatui widgets in Rust)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandSource {
    Builtin,
    InternalOnly,
    FeatureGated(String),
    Plugin(String),
    Skill(String),
}
```

### Session Types (from ECC2 session/mod.rs — proven Rust pattern)

```rust
// crates/c4l-types/src/session.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub task: String,
    pub state: SessionState,
    pub pid: Option<u32>,
    pub worktree: Option<WorktreeInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metrics: SessionMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Pending,
    Running,
    Idle,
    Completed,
    Failed,
    Stopped,
}

impl SessionState {
    /// Explicit transition validation (from ECC2 pattern)
    pub fn can_transition_to(&self, next: &Self) -> bool {
        if self == next { return true; }
        matches!(
            (self, next),
            (Self::Pending, Self::Running | Self::Failed | Self::Stopped)
            | (Self::Running, Self::Idle | Self::Completed | Self::Failed | Self::Stopped)
            | (Self::Idle, Self::Running | Self::Completed | Self::Failed | Self::Stopped)
            | (Self::Completed, Self::Stopped)
            | (Self::Failed, Self::Stopped)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub base_branch: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub tokens_used: u64,
    pub tool_calls: u64,
    pub files_changed: u32,
    pub duration_secs: u64,
    pub cost_usd: f64,
}
```

### Config Types (from RTK config.rs pattern)

```rust
// crates/c4l-config/src/lib.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawConfig {
    pub auth: AuthConfig,
    pub model: ModelConfig,
    pub shell: ShellConfig,
    pub permissions: PermissionsConfig,
    pub display: DisplayConfig,
    pub tracking: TrackingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub api_key: Option<String>,
    pub auth_token: Option<String>,
    pub base_url: Option<String>,
    pub use_bedrock: bool,
    pub use_vertex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub default_model: String,          // "claude-sonnet-4-6"
    pub fast_model: Option<String>,     // "claude-haiku-4-5"
    pub subagent_model: Option<String>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    pub shell: Option<String>,
    pub shell_prefix: Option<String>,
    pub tmpdir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsConfig {
    pub mode: c4l_types::PermissionMode,
    pub rules: Vec<c4l_types::ToolPermissionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub color: bool,
    pub theme: String,
    pub verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    pub enabled: bool,
    pub database_path: Option<PathBuf>,
    pub history_days: u32,
}

/// Load order (from RTK pattern):
/// 1. ~/.config/claw4love/config.toml (user global)
/// 2. .claw/config.toml (project local)
/// 3. Environment variables (ANTHROPIC_API_KEY, etc.)
/// 4. CLI flags (--verbose, --model, etc.)
```

## Feature Flags (Cargo features, maps from Bun's `feature()`)

```toml
# In c4l-cli/Cargo.toml
[features]
default = ["base-tools"]
base-tools = []          # BashTool, FileRead, FileEdit, FileWrite, Glob, Grep
all-tools = ["base-tools", "repl-tool", "web-tools", "task-tools"]
repl-tool = []           # REPLTool (Python/Node)
web-tools = []           # WebFetch, WebSearch
task-tools = []          # TaskCreate, TaskGet, TaskUpdate, TaskList, TaskStop
agent-tools = []         # AgentTool, SendMessage, TeamCreate, TeamDelete
mcp = []                 # MCP client/server
bridge = []              # IDE bridge
voice = []               # Voice I/O
proactive = []           # SleepTool, proactive mode
```

Maps from TypeScript feature flags:
- `PROACTIVE` → `proactive`
- `BRIDGE_MODE` → `bridge`
- `VOICE_MODE` → `voice`
- `AGENT_TRIGGERS` → `agent-tools`
- `MONITOR_TOOL` → (omit for now)

## Deliverables for Phase 0

1. `Cargo.toml` workspace with all crate stubs
2. `c4l-types` crate with all types above, compiling
3. `c4l-config` crate with config loading (TOML + env vars)
4. `c4l-cli` crate with clap skeleton (subcommands stubbed)
5. All `cargo check --workspace` passing
6. All `cargo test --workspace` passing (type serialization round-trips)
