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

## Status

Phase 0 complete. Workspace scaffolded with 13 crates.

| Crate | Status | Purpose |
|-------|--------|---------|
| c4l-types | Done | Messages, permissions, tools, commands, sessions |
| c4l-config | Done | Layered config loading (TOML + env vars + defaults) |
| c4l-cli | Done | Binary entry point with clap |
| c4l-api | Stub | Anthropic API client with streaming |
| c4l-engine | Stub | Query engine (LLM loop, tool calls) |
| c4l-tools | Stub | Tool trait and implementations |
| c4l-commands | Stub | Slash commands |
| c4l-state | Stub | SQLite session/cost persistence |
| c4l-tui | Stub | Terminal UI with ratatui |
| c4l-mcp | Stub | Model Context Protocol client |
| c4l-plugins | Stub | Plugin, skill, and hook system |
| c4l-bridge | Stub | IDE integration (VS Code, JetBrains) |
| c4l-utils | Stub | Shared utilities |

## Building

```
cargo build --workspace
cargo test --workspace
cargo run -p c4l-cli
```

## Architecture

The project is a Cargo workspace split by concern. Each crate has a single responsibility and communicates through the shared types in c4l-types.

```
User input
  -> c4l-cli (parse args, launch REPL)
    -> c4l-engine (build prompt, stream API, detect tool_use)
      -> c4l-api (HTTP streaming to Anthropic)
      -> c4l-tools (execute tools: Bash, FileRead, FileEdit, ...)
        -> c4l-state (persist session, track cost)
    -> c4l-tui (render messages, handle input)
    -> c4l-plugins (load skills, fire hooks)
    -> c4l-commands (dispatch /slash commands)
```

Config is loaded by c4l-config from three layers: user global, project local, and environment variables.

## Phases

0. Foundation (workspace, types, config, CLI shell) -- done
1. Core engine (API client, streaming, tool-call loop)
2. Essential tools (Bash, FileRead, FileEdit, FileWrite, Glob, Grep)
3. Session and state (SQLite persistence, cost tracking)
4. Terminal UI (ratatui REPL)
5. Commands (slash commands)
6. Extensions (plugins, skills, hooks, MCP)
7. Advanced (agents, worktrees, IDE bridge)
8. Polish (token optimization via filter pipeline, packaging)

Detailed plans for each phase are in the `porting/` directory.

## Research

The `research/` directory contains component-by-component analysis of:

- The original Claude Code CLI TypeScript source
- Existing Python and Rust porting attempts
- The RTK token optimization proxy
- The Everything Claude Code and Superpowers plugin ecosystems

All type definitions and architectural decisions are grounded in these findings.

## License

MIT
