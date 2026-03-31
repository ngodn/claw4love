# Claw4Love Research Index

Research findings for Claude Code CLI ecosystem — source code analysis and optimization plugins.
Goal: Plan a Rust rewrite with improvements.

## Source Code Projects

### [01 - claw-code](01-claw-code.md)
Python clean-room port of Claude Code CLI. 67 Python files, 150+ commands, 100+ tools.
Branches: main (Python), dev/rust (remote-only, Rust port in progress).
Zero external dependencies. Snapshot-based mirroring architecture.

### [01b - claw-code dev/rust branch](01b-claw-code-dev-rust.md)
Rust port in progress — only ~7% complete (481 lines, 5 files, proof-of-concept).
Compat-harness extracts 131 commands, 31 tools, 12 bootstrap phases from leaked TS source.
Also contains full leaked TypeScript source (1,884 files, 512K LOC).

### [02 - leak-claude-code](02-leak-claude-code.md)
The actual Claude Code CLI source (TypeScript/Bun). ~2,172 files, 512K+ lines.
Core: QueryEngine.ts (~46K lines), 40+ tools, 85+ commands, 140+ React/Ink components.
Branches: main, backup (original unmodified).

### [05 - rtk](05-rtk.md)
Rust Token Killer — token-optimized CLI proxy (v0.34.2). ~19,885 lines Rust, 93 modules.
60-90% token savings. 42 command modules across 9 language ecosystems.
TOML filter DSL, 3-tier parsing, SQLite tracking, multi-agent hook system.

## Optimization Plugins

### [03 - everything-claude-code](03-everything-claude-code.md)
Production plugin (v1.9.0). 30 agents, 136 skills, 60 commands, 29 hooks, 13 language rule sets.
Manifest-driven install system. ECC 2.0 Rust TUI control plane emerging.
Cross-platform: Claude Code, Cursor, Codex, OpenCode, Antigravity.

### [04 - superpowers](04-superpowers.md)
Workflow enforcement plugin (v5.0.6). 14 composable skills (3,157 lines).
Hard-gate methodology: TDD, systematic debugging, verification-before-completion.
Two-stage review gates, subagent-driven development.
Cross-platform: Claude Code, Cursor, Codex, OpenCode, Gemini CLI.

## Authentication

### [06 - OAuth Auth Flow](06-oauth-auth-flow.md)
Complete OAuth 2.0 PKCE flow for Claude subscription login (Pro/Max/Team/Enterprise).
Client ID, endpoints, scopes, token storage, refresh logic, auth priority order.
Verified from leak-claude-code/src/services/oauth/ and src/utils/auth.ts.

## Key Observations for Rust Rewrite

### From rtk (existing Rust proxy):
- **Production Rust patterns** to reuse: clap 4 CLI, anyhow/thiserror errors, rusqlite tracking
- **TOML filter DSL** — declarative 8-stage pipeline, 60+ built-in filters compiled by build.rs
- **3-tier parsing** — Full/Degraded/Passthrough fallback for robustness
- **Hook architecture** — Multi-agent support (Claude, Cursor, Windsurf, Cline, Copilot)
- **Performance profile** — LTO + strip + abort = <5MB binary, <10ms startup
- **Already solves** token optimization; rewrite should integrate/extend this

### From leak-claude-code (the actual CLI):
- **QueryEngine.ts** is the heart (~46K lines) — LLM streaming, tool-call loops, thinking mode
- **Tool system** is well-abstracted — each tool has call/permissions/render interface
- **React/Ink** for terminal UI — Rust equivalent: Ratatui
- **Bun runtime** with feature flag dead-code elimination
- **Commander.js** for CLI — Rust equivalent: clap
- **Zod** for validation — Rust equivalent: serde + custom validation
- **30+ external dependencies** — most have Rust equivalents

### From claw-code (Python port):
- Already mapped architecture to 24 subsystems
- Frozen dataclasses → Rust structs trivially
- Token-based routing for prompt→command/tool matching
- Permission system as blocklist (frozenset/tuple)
- dev/rust branch already started

### From plugins (ECC + superpowers):
- Skill/agent/hook/command system is the extension model
- Manifest-driven installation is the configuration approach
- Hard-gate workflow enforcement is the quality methodology
- Session adapters for cross-platform compatibility
- Continuous learning pipeline for skill evolution
- ECC 2.0 already uses Rust (ratatui, rusqlite, clap, tokio, git2)
