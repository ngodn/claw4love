# Phases 7-8: Advanced Features & Polish

## Phase 7: Agents, Bridge, MCP, Worktrees

### Sub-Agent System

Maps from: `src/tools/AgentTool/`, `src/coordinator/`

```rust
// crates/c4l-tools/src/agent.rs (added to existing crate)

/// AgentTool — spawn a fresh sub-agent with isolated context
/// Maps from: AgentTool in TypeScript
///
/// Key design: Each sub-agent gets its own QueryEngine instance
/// with a separate message history. Results flow back via channel.
pub struct AgentTool;

/// Agent definition loaded from .claude/agents/ or plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,     // tool names this agent can use
    pub model: Option<String>,  // model override
    pub prompt: String,         // system prompt injection
}

/// Agent execution modes
pub enum AgentIsolation {
    /// Run in current process, separate message history
    InProcess,
    /// Run in git worktree (isolated filesystem)
    Worktree { path: PathBuf, branch: String },
}
```

### Git Worktree Integration

Maps from: `src/tools/EnterWorktreeTool/`, Superpowers `using-git-worktrees` skill

```rust
// crates/c4l-utils/src/worktree.rs

use git2::Repository;

/// Create an isolated git worktree for agent work
/// Maps from: EnterWorktreeTool + Superpowers worktree skill
pub fn create_worktree(
    repo: &Repository,
    branch_name: &str,
    base_dir: &Path,  // .worktrees/ or worktrees/
) -> anyhow::Result<WorktreeInfo>;

/// Clean up a worktree
pub fn remove_worktree(
    repo: &Repository,
    path: &Path,
) -> anyhow::Result<()>;

/// Verify worktree directory is gitignored
pub fn ensure_gitignored(repo: &Repository, dir_name: &str) -> anyhow::Result<()>;
```

### MCP Client

Maps from: `src/services/mcp/client.ts` (3,348 lines)

```rust
// crates/c4l-mcp/src/lib.rs

/// MCP (Model Context Protocol) client
/// Connects to MCP servers defined in .mcp.json
pub struct McpClient {
    connections: HashMap<String, McpConnection>,
}

pub struct McpConnection {
    server_name: String,
    transport: McpTransport,
}

pub enum McpTransport {
    Stdio { command: String, args: Vec<String> },
    Http { url: String },
}

/// MCP server config (from .mcp.json)
#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transport_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl McpClient {
    /// Discover and connect to MCP servers
    pub async fn from_config(path: &Path) -> anyhow::Result<Self>;

    /// List available tools from all connected servers
    pub async fn list_tools(&self) -> Vec<McpToolDef>;

    /// Execute a tool on a specific server
    pub async fn call_tool(
        &self,
        server: &str,
        tool: &str,
        input: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;

    /// List resources from a server
    pub async fn list_resources(&self, server: &str) -> Vec<McpResource>;

    /// Read a resource
    pub async fn read_resource(
        &self,
        server: &str,
        uri: &str,
    ) -> anyhow::Result<String>;
}
```

### IDE Bridge

Maps from: `src/bridge/` (16 files, ~5K lines)

```rust
// crates/c4l-bridge/src/lib.rs

/// IDE bridge — bidirectional communication with VS Code, JetBrains
/// Feature-gated: #[cfg(feature = "bridge")]
///
/// Protocol: JSON messages over stdin/stdout or WebSocket
/// Authentication: JWT tokens
///
/// Capabilities:
/// - Route permission prompts to IDE
/// - Show diffs in IDE
/// - File watching sync
/// - Session state sync

pub struct BridgeServer {
    // ... WebSocket server for IDE connections
}

// NOTE: Bridge is complex (5K lines in TS). Implement as a thin
// layer initially — just enough for VS Code extension compatibility.
// Full implementation is Phase 7+ priority.
```

---

## Phase 8: Token Optimization & Polish

### RTK Integration

The key improvement over vanilla Claude Code — built-in token optimization.

```rust
// crates/c4l-utils/src/token_filter.rs

/// Integrate RTK's TOML filter pipeline directly into tool output
///
/// Instead of RTK as external proxy, embed the filtering:
/// 1. BashTool executes command
/// 2. Raw output passed through TOML filter pipeline
/// 3. Filtered output sent to LLM (60-90% fewer tokens)
/// 4. Token savings tracked in SQLite
///
/// This is the KEY IMPROVEMENT over TypeScript Claude Code.

/// TOML filter pipeline (ported from RTK's core/toml_filter.rs)
/// 8-stage pipeline:
/// 1. strip_ansi
/// 2. replace (regex substitutions)
/// 3. match_output (short-circuit on known patterns)
/// 4. strip/keep_lines (regex line filtering)
/// 5. truncate_lines_at (per-line truncation)
/// 6. head/tail_lines
/// 7. max_lines (absolute cap)
/// 8. on_empty (message if result empty)
pub struct FilterPipeline {
    filters: Vec<CompiledFilter>,
}

/// Load filters from:
/// 1. Built-in (embedded via build.rs, like RTK)
/// 2. User global (~/.config/claw4love/filters.toml)
/// 3. Project local (.claw/filters.toml)
pub fn load_filters() -> FilterPipeline;

/// Apply filter to command output
pub fn filter_output(
    pipeline: &FilterPipeline,
    command: &str,
    output: &str,
) -> FilterResult;

/// RTK's 3-tier result pattern
pub enum FilterResult {
    Full(FilteredOutput),
    Partial(FilteredOutput, Vec<String>),
    Passthrough(String),
}

pub struct FilteredOutput {
    pub content: String,
    pub original_tokens: usize,
    pub filtered_tokens: usize,
    pub savings_pct: f64,
}
```

### Built-in Improvements Over TypeScript Version

| Area | TypeScript | Rust (claw4love) | Source |
|------|-----------|-------------------|--------|
| Token optimization | None built-in | RTK filter pipeline embedded | RTK |
| Startup time | ~2s (Bun + React) | <50ms (native binary) | Rust |
| Binary size | ~50MB (node_modules) | <10MB (single binary) | Rust |
| Memory usage | ~200MB (V8 heap) | <30MB | Rust |
| Session persistence | JSON files | SQLite (queryable, atomic) | ECC2 |
| Config validation | Zod runtime | serde compile-time + runtime | Rust |
| Skill/hook system | JS-based | Multi-platform (ECC+Superpowers compatible) | Both |
| Concurrency | Single-threaded + async | True multi-threaded + async | tokio |
| Error handling | try/catch (lossy) | Result<T, E> (typed, propagated) | Rust |
| Feature flags | Runtime (Bun feature()) | Compile-time (Cargo features) | Rust |

### Final Polish Checklist

- [ ] Shell completion generation (clap_complete)
- [ ] Man page generation (clap_mangen)
- [ ] Homebrew formula
- [ ] Debian/RPM packaging (cargo-deb, cargo-rpm)
- [ ] Cross-compilation (Linux, macOS, Windows)
- [ ] Integration tests against real Anthropic API
- [ ] Performance benchmarks (startup, memory, throughput)
- [ ] Plugin compatibility with existing ECC/Superpowers plugins
- [ ] CLAUDE.md support (project memory)
- [ ] Settings migration from existing ~/.claude/

## Deliverables for Phases 7-8

### Phase 7
1. AgentTool with in-process sub-agent execution
2. Git worktree creation/cleanup (via git2)
3. MCP client with STDIO transport
4. Basic IDE bridge (WebSocket, permission routing)

### Phase 8
1. TOML filter pipeline (ported from RTK)
2. Built-in filters for common commands (60+ from RTK)
3. Token savings tracking and reporting
4. Shell completions and man pages
5. Cross-platform packaging
6. Performance benchmarks
