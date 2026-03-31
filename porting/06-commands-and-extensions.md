# Phases 5-6: Commands, Plugins, Skills, Hooks

## Phase 5: Slash Commands

Maps from: `src/commands.ts` (400+ lines), `src/commands/` (85 directories)

### Command Trait

```rust
// crates/c4l-commands/src/lib.rs

use c4l_types::{CommandType, Message};
use c4l_state::SharedAppState;

/// Result of a command execution
pub enum CommandResult {
    /// Text output (LocalCommand)
    Text(String),
    /// Formatted prompt to send to LLM (PromptCommand)
    Prompt { prompt: String, tools: Vec<String> },
    /// No output (side effect only)
    None,
}

/// Slash command trait
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> Vec<&str> { vec![] }
    fn description(&self) -> &str;
    fn command_type(&self) -> CommandType;

    /// Execute the command
    fn execute(
        &self,
        args: &str,
        state: &SharedAppState,
    ) -> anyhow::Result<CommandResult>;
}

pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
}
```

### Priority Commands (implement first)

| Command | Type | Complexity | Maps From |
|---------|------|------------|-----------|
| /help | Local | Low | commands/help/ |
| /clear | Local | Low | commands/clear/ |
| /exit | Local | Low | commands/exit/ |
| /cost | Local | Low | commands/cost/ |
| /compact | Local | Medium | commands/compact/ |
| /config | Local | Medium | commands/config/ |
| /commit | Prompt | Medium | commands/commit.js |
| /review | Prompt | Medium | commands/review.js |
| /diff | Local | Low | commands/diff/ |
| /status | Local | Low | commands/status/ |
| /resume | Local | Medium | commands/session/ |
| /memory | Local | Medium | commands/memory/ |
| /vim | Local | Low | commands/vim/ |
| /plan | Prompt | Low | commands/plan/ |
| /init | Local | Medium | commands/init.js |

### Later Commands (implement as needed)

| Command | Notes |
|---------|-------|
| /mcp | MCP server management (Phase 6) |
| /plugin | Plugin management (Phase 6) |
| /skills | Skill management (Phase 6) |
| /agents | Agent management (Phase 7) |
| /teleport | Remote session (Phase 7) |
| /share | Share session (Phase 7) |
| /doctor | Environment diagnostics |
| /ide | IDE integration (Phase 7) |

---

## Phase 6: Extension System (Plugins, Skills, Hooks)

Maps from:
- TypeScript: `src/services/plugins/`, `src/skills/`, `src/hooks/`
- ECC: 30 agents, 136 skills, 29 hooks, 60 commands
- Superpowers: 14 skills with hard-gate methodology

### Plugin System

```rust
// crates/c4l-plugins/src/plugin.rs

use std::path::PathBuf;

/// Plugin manifest (loaded from plugin directory)
/// Maps from: .claude-plugin/plugin.json pattern in ECC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,       // entry point script
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<PathBuf>,     // hooks.json location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_dir: Option<PathBuf>,
}

/// Plugin discovery
/// Scans: ~/.claude/plugins/, .claude/plugins/ (project)
pub fn discover_plugins(paths: &[PathBuf]) -> Vec<PluginManifest>;
```

### Skill System

```rust
// crates/c4l-plugins/src/skill.rs

/// Skill definition — loaded from SKILL.md or README.md files
/// Maps from: Superpowers SKILL.md format + ECC skills/ format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub description: String,  // "Use when [triggering conditions]"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,  // Full skill content (loaded on demand)
}

/// Parse YAML frontmatter from markdown skill file
/// Format:
/// ---
/// name: skill-name
/// description: "Use when ..."
/// ---
/// [markdown content]
pub fn parse_skill_file(path: &Path) -> anyhow::Result<SkillManifest>;

/// Skill discovery
/// Scans: ~/.claude/skills/, .claude/skills/, plugin skills dirs
pub fn discover_skills(paths: &[PathBuf]) -> Vec<SkillManifest>;
```

### Hook System

```rust
// crates/c4l-plugins/src/hook.rs

/// Hook event types
/// Maps from: TypeScript hook system + ECC hooks.json + Superpowers hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    SessionStart,
    SessionEnd,
    Stop,
    PreCompact,
}

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    pub event: HookEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,    // tool name pattern
    pub command: String,            // shell command to execute
    #[serde(default)]
    pub r#async: bool,              // run asynchronously
}

/// Hook execution result
pub struct HookResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    /// Parsed output (for system message injection)
    pub additional_context: Option<String>,
}

/// Hook input passed to hook scripts (as JSON on stdin)
/// Maps from: ECC hooks/README.md HookInput schema
#[derive(Debug, Serialize)]
pub struct HookInput {
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<serde_json::Value>,
}

/// Load hooks from hooks.json files
/// Sources: ~/.claude/hooks.json, .claude/hooks.json, plugin hooks
pub fn load_hooks(paths: &[PathBuf]) -> Vec<HookDef>;

/// Execute matching hooks for an event
/// PreToolUse: exit code 2 = block tool execution
pub async fn execute_hooks(
    hooks: &[HookDef],
    event: HookEvent,
    input: &HookInput,
) -> Vec<HookResult>;
```

### Memory System (CLAUDE.md)

```rust
// crates/c4l-plugins/src/memory.rs

/// Memory file discovery and loading
/// Maps from: src/memdir/ in TypeScript
///
/// Hierarchy:
/// 1. Project: ./CLAUDE.md
/// 2. Project config: .claude/CLAUDE.md
/// 3. User global: ~/.claude/CLAUDE.md
/// 4. User referenced: @file.md includes
pub fn load_memory_files(project_root: &Path) -> Vec<MemoryFile>;

pub struct MemoryFile {
    pub path: PathBuf,
    pub content: String,
    pub scope: MemoryScope,
}

pub enum MemoryScope {
    Project,
    ProjectConfig,
    UserGlobal,
    Referenced(String),  // included via @path
}
```

## Deliverables for Phases 5-6

### Phase 5
1. Command trait and CommandRegistry
2. 15 priority commands implemented
3. Command dispatch from REPL input (detect `/` prefix)

### Phase 6
1. Plugin manifest loading and discovery
2. Skill file parsing (YAML frontmatter + markdown)
3. SkillTool integration (load skill content into conversation)
4. Hook system (pre/post tool use, session lifecycle)
5. Hook execution with exit code handling (2 = block)
6. Memory system (CLAUDE.md hierarchy loading)
7. Tests: plugin discovery, skill parsing, hook execution
