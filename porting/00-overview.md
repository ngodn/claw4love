# Claw4Love: Claude Code CLI Rust Rewrite — Overview

## Goal

Rewrite Claude Code CLI in Rust with improvements from all studied projects:
- **leak-claude-code** — the actual TypeScript implementation (reference architecture)
- **claw-code** — Python port (subsystem mapping) + dev/rust (proof-of-concept)
- **rtk** — production Rust proxy (reusable patterns, 19K LOC)
- **everything-claude-code** — plugin system, agents, skills, hooks, session management
- **superpowers** — workflow enforcement, skill design patterns

## Project Name: `claw4love`

## Non-Goals (for now)

- Full React/Ink UI port (use ratatui instead, incrementally)
- Telemetry/analytics (GrowthBook, OpenTelemetry) — add later
- Voice mode, buddy system, coordinator mode — add later
- Web UI — out of scope

## Architecture Principles

1. **Crate workspace** — separate crates per subsystem (like RTK's modular structure)
2. **Async-first** — tokio runtime (like ECC2, unlike RTK which is sync)
3. **Trait-based tools** — each tool implements a Tool trait (maps from TypeScript Tool interface)
4. **Feature flags** — Cargo features for conditional compilation (maps from Bun's `feature()`)
5. **Graceful degradation** — RTK's 3-tier ParseResult pattern everywhere
6. **SQLite state** — sessions, tracking, config (proven in both RTK and ECC2)
7. **Plugin-ready** — skill/hook/command extension points from day one

## Tech Stack (Verified from RTK + ECC2)

| Need | Crate | Version | Source |
|------|-------|---------|--------|
| CLI parsing | clap | 4.x (derive) | RTK |
| Async runtime | tokio | 1.x (full) | ECC2 |
| Error handling | anyhow + thiserror | 1.0 / 2.0 | Both |
| Serialization | serde + serde_json | 1.x | Both |
| Config files | toml | 0.8 | Both |
| SQLite | rusqlite | 0.32 (bundled) | ECC2 |
| Terminal UI | ratatui + crossterm | 0.29 / 0.28 | ECC2 |
| HTTP client | reqwest | 0.12 | New (replaces ureq for async) |
| Git | git2 | 0.20 | ECC2 |
| Regex | regex + lazy_static | 1.x / 1.4 | RTK |
| Logging | tracing + tracing-subscriber | 0.1 / 0.3 | ECC2 |
| Time | chrono | 0.4 (serde) | Both |
| UUID | uuid | 1.x (v4) | ECC2 |
| Dirs | dirs | 6.x | ECC2 |
| File watching | notify | 7.x | New (replaces chokidar) |
| Process exec | tokio::process | (bundled) | New (replaces execa/node-pty) |
| Markdown | pulldown-cmark | 0.12 | New (replaces marked) |
| Diff | similar | 2.x | New (replaces diff) |
| Glob | globset | 0.4 | New (replaces picomatch) |
| Fuzzy search | nucleo | 0.5 | New (replaces fuse.js) |
| YAML | serde_yaml | 0.9 | New (replaces yaml) |
| Semver | semver | 1.x | New |
| WebSocket | tokio-tungstenite | 0.24 | New (replaces ws) |
| MCP SDK | TBD | — | Port or wrap @modelcontextprotocol/sdk |
| Anthropic SDK | TBD | — | Port or wrap @anthropic-ai/sdk |

## Release Profile (from RTK, proven)

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Target: <10MB binary, <50ms startup.

## Phased Delivery

| Phase | Name | Deliverable | Est. Crates |
|-------|------|-------------|-------------|
| 0 | Foundation | Workspace, types, config, CLI shell | 4 |
| 1 | Core Engine | API client, streaming, tool-call loop | 3 |
| 2 | Essential Tools | Bash, FileRead, FileEdit, FileWrite, Glob, Grep | 1 |
| 3 | Session & State | Session store, history, transcript, cost tracking | 2 |
| 4 | Terminal UI | REPL screen, message rendering, input handling | 1 |
| 5 | Commands | Slash commands (/commit, /review, /help, etc.) | 1 |
| 6 | Extensions | Plugins, skills, hooks, MCP client | 3 |
| 7 | Advanced | Agents, worktrees, bridge/IDE, remote modes | 2 |
| 8 | Polish | Token optimization (RTK integration), telemetry | 1 |

See individual phase documents for details.

## Porting Documents

- [01-workspace-and-types.md](01-workspace-and-types.md) — Phase 0: Workspace layout, core types
- [02-core-engine.md](02-core-engine.md) — Phase 1: API client, QueryEngine, streaming
- [03-tool-system.md](03-tool-system.md) — Phase 2: Tool trait, essential tools
- [04-session-and-state.md](04-session-and-state.md) — Phase 3: Persistence, state management
- [05-terminal-ui.md](05-terminal-ui.md) — Phase 4: Ratatui REPL
- [06-commands-and-extensions.md](06-commands-and-extensions.md) — Phases 5-6: Commands, plugins, skills
- [07-advanced-and-polish.md](07-advanced-and-polish.md) — Phases 7-8: Agents, bridge, RTK integration
