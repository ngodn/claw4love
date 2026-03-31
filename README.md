# claw4love

A ground-up rewrite of Claude Code CLI in Rust. Faster startup, smaller binary, built-in token optimization, and a proper extension system.

## Why

Claude Code CLI is 512K lines of TypeScript running on Bun. It works, but:

- 2+ second startup time
- 50MB+ installed size (node_modules)
- 200MB+ memory usage at runtime
- No built-in token optimization (tools like RTK exist as external proxies)
- Extension system bolted on after the fact

claw4love targets: under 10MB binary, under 50ms startup, under 30MB memory, with token-optimized tool output baked in from day one.

## Quick Start

### Prerequisites

- Rust toolchain (rustup.rs)
- ripgrep (`rg`) for the Grep tool
- git

### Build

```
git clone https://github.com/ngodn/claw4love.git
cd claw4love
cargo build --workspace
```

For a release build (optimized, <10MB binary):

```
cargo build --release
```

The binary will be at `target/release/claw4love`.

### Authenticate

Option 1: Log in with your Claude subscription (Pro/Max/Team/Enterprise):

```
cargo run -p c4l-cli -- login
```

This opens a browser for OAuth authentication. Tokens are stored in `~/.claude/.credentials.json` (same location as the official CLI).

Option 2: Use an API key:

```
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

### Run

Interactive REPL (TUI):

```
cargo run -p c4l-cli
```

Single prompt (non-interactive, prints to stdout):

```
cargo run -p c4l-cli -- --prompt "explain what this project does"
```

With a specific model:

```
cargo run -p c4l-cli -- --model claude-opus-4-6
```

### Commands

```
claw4love                     # Interactive REPL
claw4love --prompt "..."      # Single prompt, output to stdout
claw4love --model opus ...    # Use a specific model
claw4love login               # Authenticate with Claude subscription
claw4love logout              # Clear stored credentials
claw4love config              # Show current configuration
claw4love doctor              # Run environment diagnostics
claw4love sessions            # List past sessions
claw4love cost                # Show token usage and cost
claw4love cost --all          # Show costs for all sessions (last 30 days)
claw4love resume <id>         # Resume a previous session
claw4love version             # Show version info
```

### Slash Commands (inside REPL)

```
/help                         # Show available commands
/clear                        # Clear conversation history
/exit                         # Exit (also Ctrl+C)
/cost                         # Show session cost
/status                       # Show session info
/config                       # Show current config
/commit                       # Generate commit message from staged changes
/review                       # Review code changes
/plan <task>                  # Create an implementation plan
/diff                         # Show git diff
/compact                      # Compact conversation to save context
```

### Test

```
cargo test --workspace
```

Currently 164 tests across all crates.

### Configuration

Config is loaded in this order (later overrides earlier):

1. Built-in defaults
2. User global: `~/.config/claw4love/config.toml`
3. Project local: `.claw4love/config.toml`
4. Environment variables

Example config.toml:

```toml
[auth]
api_key = "sk-ant-..."   # or use ANTHROPIC_API_KEY env var

[model]
default_model = "claude-sonnet-4-6"
fast_model = "claude-haiku-4-5"

[display]
color = true
theme = "dark"
verbose = false

[tracking]
enabled = true
history_days = 90
```

Environment variables (same as official Claude Code CLI):

```
ANTHROPIC_API_KEY          # API key for direct access
ANTHROPIC_MODEL            # Override default model
ANTHROPIC_BASE_URL         # Custom API endpoint
CLAUDE_CODE_SHELL          # Shell to use for Bash tool
CLAUDE_CONFIG_DIR          # Override config directory (default: ~/.claude)
```

## Status

All 8 phases scaffolded. 13 crates, 164 tests passing.

| Crate | Status | Purpose |
|-------|--------|---------|
| c4l-types | Done | Messages, permissions, tools, commands, sessions |
| c4l-config | Done | Layered config loading (TOML + env vars + defaults) |
| c4l-cli | Done | Binary with login, doctor, sessions, REPL, oneshot |
| c4l-api | Done | Anthropic API client with SSE streaming, retry, OAuth |
| c4l-engine | Done | Query engine with tool-call loop and event streaming |
| c4l-tools | Done | Tool trait, registry, and 6 essential tools |
| c4l-state | Done | SQLite session store, cost tracking, shared app state |
| c4l-commands | Done | Slash command trait, registry, 11 built-in commands |
| c4l-tui | Done | Ratatui REPL with streaming, input, permissions |
| c4l-mcp | Done | MCP client with STDIO transport and JSON-RPC |
| c4l-plugins | Done | Skill parsing, hook execution, memory loading, plugin discovery |
| c4l-bridge | Done | IDE bridge protocol types |
| c4l-utils | Done | Token filter pipeline, ANSI stripping, git worktrees |

### Tools

Bash, Read (FileRead), Edit (FileEdit), Write (FileWrite), Glob, Grep

### Authentication

- OAuth login with Claude Pro/Max/Team/Enterprise subscriptions (`claw4love login`)
- API key via environment variable or config file
- Tokens stored in `~/.claude/.credentials.json` (compatible with official CLI)
- Auto-refresh before expiry

## Architecture

The project is a Cargo workspace split by concern. Each crate has a single responsibility and communicates through the shared types in c4l-types.

```
User input
  -> c4l-cli (parse args, launch REPL or oneshot)
    -> c4l-engine (build prompt, stream API, detect tool_use)
      -> c4l-api (HTTP streaming to Anthropic, OAuth or API key)
      -> c4l-tools (execute tools: Bash, FileRead, FileEdit, ...)
        -> c4l-state (persist session, track cost in SQLite)
    -> c4l-tui (render messages, handle input, permission prompts)
    -> c4l-plugins (load skills, fire hooks, read CLAUDE.md)
    -> c4l-commands (dispatch /slash commands)
```

## Project Layout

```
claw4love/
  Cargo.toml          # Workspace root
  crates/             # 13 crates
  playground/         # Prototyping area
  porting/            # Phase-by-phase implementation plans
  research/           # Source analysis of Claude Code ecosystem
  README.md
  RUST.md             # Rust coding guidelines
```

## Research

The `research/` directory contains component-by-component analysis of:

- The original Claude Code CLI TypeScript source (512K LOC)
- Existing Python and Rust porting attempts
- The RTK token optimization proxy (19K LOC Rust)
- The Everything Claude Code and Superpowers plugin ecosystems
- OAuth authentication flow (endpoints, scopes, token storage)

All type definitions and architectural decisions are grounded in these findings.

## License

MIT
