# Phase 2: Tool System — Trait Definition & Essential Tools

## What This Phase Delivers

The Tool trait (maps from TypeScript Tool interface in Tool.ts) and the 6 essential tools that make Claude Code functional: Bash, FileRead, FileEdit, FileWrite, Glob, Grep.

## The Tool Trait

Maps from: `src/Tool.ts` (795 lines) — the exact interface every tool implements.

```rust
// crates/c4l-tools/src/trait.rs

use async_trait::async_trait;
use c4l_types::{
    PermissionResult, ToolInputSchema, ToolUseContext, ValidationResult,
};
use serde_json::Value;

/// The result of a tool execution
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Structured output data
    pub data: Value,
    /// Whether to include in API response as tool_result
    pub is_error: bool,
    /// Human-readable summary for display
    pub display: Option<String>,
}

/// Core tool trait — every tool implements this
/// Maps from: TypeScript Tool<Input, Output, Progress> interface
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name as registered with the API
    fn name(&self) -> &str;

    /// Alternative names (e.g., "Read" for "FileReadTool")
    fn aliases(&self) -> Vec<&str> { vec![] }

    /// JSON schema for tool input parameters
    fn input_schema(&self) -> ToolInputSchema;

    /// System prompt injection — added to system prompt when tool is available
    /// Maps from: Tool.prompt() in TypeScript
    async fn prompt(&self) -> String;

    /// Human-readable description for the API
    async fn description(&self, input: &Value) -> String;

    /// Execute the tool
    /// Maps from: Tool.call() in TypeScript
    async fn call(
        &self,
        input: Value,
        context: &ToolUseContext,
    ) -> anyhow::Result<ToolResult>;

    /// Check if this tool use is allowed
    /// Maps from: Tool.checkPermissions() in TypeScript
    async fn check_permissions(
        &self,
        input: &Value,
        context: &ToolUseContext,
    ) -> PermissionResult {
        PermissionResult::Allow { updated_input: None }
    }

    /// Validate input before execution
    fn validate_input(&self, input: &Value) -> ValidationResult {
        ValidationResult::Ok
    }

    /// Can this tool run concurrently with others?
    fn is_concurrency_safe(&self, _input: &Value) -> bool { false }

    /// Does this tool only read (not modify) state?
    fn is_read_only(&self, _input: &Value) -> bool { false }

    /// Could this tool cause destructive changes?
    fn is_destructive(&self, _input: &Value) -> bool { false }

    /// Activity description for status display
    fn activity_description(&self, input: &Value) -> Option<String> { None }
}

/// Tool registry — holds all available tools
/// Maps from: src/tools.ts registration pattern
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self { Self { tools: vec![] } }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.iter().find(|t| {
            t.name() == name || t.aliases().contains(&name)
        }).map(|t| t.as_ref())
    }

    pub fn all(&self) -> &[Box<dyn Tool>] { &self.tools }

    /// Build API tool definitions for the request
    pub async fn api_tool_defs(&self) -> Vec<c4l_api::ApiToolDef> {
        let mut defs = vec![];
        for tool in &self.tools {
            defs.push(c4l_api::ApiToolDef {
                name: tool.name().to_string(),
                description: tool.description(&Value::Null).await,
                input_schema: tool.input_schema(),
            });
        }
        defs
    }
}
```

## Essential Tool Implementations

### BashTool

Maps from: `src/tools/BashTool/BashTool.ts`

```rust
// crates/c4l-tools/src/bash.rs

pub struct BashTool;

// Input schema:
// { command: string, description?: string, timeout?: number }
//
// Permission logic:
// - Check command against allow/deny rules
// - Pattern matching: "Bash(git *)" allows all git commands
// - Destructive detection: rm -rf, git reset --hard, etc.
//
// Execution:
// - Spawn shell process (from config.shell or default)
// - Capture stdout + stderr
// - Respect timeout (default 120s, max 600s)
// - Propagate exit code
// - Strip ANSI if needed
//
// IMPROVEMENT over TypeScript:
// - Integrate RTK's token-optimized filtering on output
// - Use RTK's TOML filter pipeline for known commands
// - Track token savings (RTK's tracking.rs pattern)
```

### FileReadTool

Maps from: `src/tools/FileReadTool/FileReadTool.ts`

```rust
// crates/c4l-tools/src/file_read.rs

pub struct FileReadTool;

// Input schema:
// { file_path: string, offset?: number, limit?: number, pages?: string }
//
// Capabilities:
// - Read text files with line numbers (cat -n format)
// - Read images (return base64 for API)
// - Read PDFs (extract text, page ranges)
// - Read Jupyter notebooks (.ipynb)
// - Respect offset/limit for large files
//
// Permission: Always allow (read-only)
// is_read_only: true
// is_concurrency_safe: true
```

### FileEditTool

Maps from: `src/tools/FileEditTool/FileEditTool.ts`

```rust
// crates/c4l-tools/src/file_edit.rs

pub struct FileEditTool;

// Input schema:
// { file_path: string, old_string: string, new_string: string, replace_all?: bool }
//
// Logic:
// - Find old_string in file (must be unique unless replace_all)
// - Replace with new_string
// - Fail if old_string not found or not unique
// - Preserve file permissions and encoding
//
// Permission: Prompt user (modifies files)
// is_destructive: false (reversible edit)
```

### FileWriteTool

Maps from: `src/tools/FileWriteTool/FileWriteTool.ts`

```rust
// crates/c4l-tools/src/file_write.rs

pub struct FileWriteTool;

// Input schema:
// { file_path: string, content: string }
//
// Logic:
// - Write content to file (create or overwrite)
// - Create parent directories if needed
// - Fail if file exists and wasn't read first in conversation
//
// Permission: Prompt user (creates/overwrites files)
// is_destructive: true (overwrites)
```

### GlobTool

Maps from: `src/tools/GlobTool/GlobTool.ts`

```rust
// crates/c4l-tools/src/glob.rs

pub struct GlobTool;

// Input schema:
// { pattern: string, path?: string }
//
// Logic:
// - Use globset crate for pattern matching
// - Respect .gitignore (use ignore crate, like RTK)
// - Return matching file paths sorted by modification time
// - Limit results (default 1000)
//
// Permission: Always allow (read-only)
// is_read_only: true
// is_concurrency_safe: true
```

### GrepTool

Maps from: `src/tools/GrepTool/GrepTool.ts`

```rust
// crates/c4l-tools/src/grep.rs

pub struct GrepTool;

// Input schema:
// { pattern: string, path?: string, glob?: string, type?: string,
//   output_mode?: string, context?: number, head_limit?: number, ... }
//
// Logic:
// - Shell out to `rg` (ripgrep) — same as TypeScript version
// - Parse output into structured results
// - Support output modes: content, files_with_matches, count
// - Respect head_limit (default 250)
//
// IMPROVEMENT: Use RTK's grep_cmd.rs grouping for token-optimized output
//
// Permission: Always allow (read-only)
// is_read_only: true
// is_concurrency_safe: true
```

## Additional Tools (implement after essentials)

| Priority | Tool | Complexity | Notes |
|----------|------|------------|-------|
| High | TodoWriteTool | Low | Structured task management |
| High | AskUserQuestionTool | Low | Prompt user for input |
| High | NotebookEditTool | Medium | Jupyter cell editing |
| Medium | WebFetchTool | Medium | HTTP fetch + HTML→text |
| Medium | WebSearchTool | Medium | Web search API |
| Medium | AgentTool | High | Spawn sub-agents |
| Medium | SendMessageTool | Medium | Inter-agent communication |
| Medium | SkillTool | Medium | Execute registered skills |
| Low | EnterPlanModeTool | Low | Permission mode switch |
| Low | ExitPlanModeTool | Low | Permission mode switch |
| Low | EnterWorktreeTool | Medium | Git worktree creation |
| Low | ExitWorktreeTool | Medium | Git worktree cleanup |
| Low | TaskCreate/Get/Update/List/Stop | Medium | Background task management |
| Low | ToolSearchTool | Low | Discover deferred tools |
| Low | MCPTool | High | MCP server invocation |
| Low | LSPTool | High | Language Server Protocol |
| Low | ConfigTool | Low | Settings management |

## Deliverables for Phase 2

1. Tool trait definition in `c4l-tools`
2. ToolRegistry with registration and lookup
3. BashTool with shell execution, timeout, exit code propagation
4. FileReadTool with text, image, offset/limit support
5. FileEditTool with unique string replacement
6. FileWriteTool with create/overwrite
7. GlobTool with gitignore-aware matching
8. GrepTool with ripgrep integration
9. Integration tests for each tool (real filesystem operations)
10. Permission check integration with ToolPermissionContext
