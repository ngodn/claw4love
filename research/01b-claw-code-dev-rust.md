# Claw-Code dev/rust Branch: Rust Port Progress Analysis

## Executive Summary

The `dev/rust` branch contains two components:

1. **Leaked Claude Code TypeScript Source** (`src/` directory) — 1,884 files, 512,664 lines
2. **Rust Port Foundation** (`rust/` directory) — v0.1.0, proof-of-concept, 481 lines across 5 files

**Overall Porting Progress: ~5-10% (scaffolding phase only)**

---

## 1. Directory Structure

```
claw-code (dev/rust)/
├── rust/                          # THE RUST PORT
│   ├── Cargo.toml                # Workspace config (v0.1.0, MIT)
│   ├── README.md                 # Milestone documentation
│   └── crates/
│       ├── rusty-claude-cli/     # CLI entry point (55 lines)
│       │   └── src/main.rs
│       ├── runtime/              # Bootstrap phases (57 lines)
│       │   └── src/lib.rs
│       ├── commands/             # Command registry types (30 lines)
│       │   └── src/lib.rs
│       ├── tools/                # Tool registry types (29 lines)
│       │   └── src/lib.rs
│       └── compat-harness/       # Upstream source extraction (309 lines)
│           └── src/lib.rs
├── src/                          # LEAKED TYPESCRIPT SOURCE (1,884 files, 512K LOC)
├── .port_sessions/               # 104 session telemetry JSONs
├── README.md                     # Architecture overview
├── 2026-03-09-is-legal-*.md      # Legal/ethics article on AI reimplementation
└── assets/ .github/ tests/
```

---

## 2. Rust Code Deep Dive

### 2.1 CLI Entry: `rusty-claude-cli/src/main.rs` (55 lines)

```rust
fn main()              // CLI dispatcher (dump-manifests | bootstrap-plan | --help)
fn dump_manifests()    // Extracts & counts: 131 commands, 31 tools, 12 phases
fn print_bootstrap_plan() // Lists 12 startup phases
fn print_help()        // Manual help text
```

**Status:** Placeholder CLI only. No clap, no actual execution logic.

### 2.2 Bootstrap Runtime: `runtime/src/lib.rs` (57 lines)

**BootstrapPhase enum (12 phases):**
```
CliEntry → FastPathVersion → StartupProfiler → SystemPromptFastPath
→ ChromeMcpFastPath → DaemonWorkerFastPath → BridgeFastPath
→ DaemonFastPath → BackgroundSessionFastPath → TemplateFastPath
→ EnvironmentRunnerFastPath → MainRuntime
```

**BootstrapPlan struct:**
- `claude_code_default()` — hardcoded 12-phase sequence
- `from_phases()` — deduplicates custom plans
- Informational only (no dispatch implemented)

### 2.3 Command Registry: `commands/src/lib.rs` (30 lines)

```rust
struct CommandManifestEntry { name: String, source: CommandSource }
enum CommandSource { Builtin, InternalOnly, FeatureGated }
struct CommandRegistry { entries: Vec<CommandManifestEntry> }
```

Data container only. No execution logic.

### 2.4 Tool Registry: `tools/src/lib.rs` (29 lines)

```rust
struct ToolManifestEntry { name: String, source: ToolSource }
enum ToolSource { Base, Conditional }
struct ToolRegistry { entries: Vec<ToolManifestEntry> }
```

Same pattern as CommandRegistry. Data container only.

### 2.5 Compat Harness: `compat-harness/src/lib.rs` (309 lines) — THE ONLY SUBSTANTIAL MODULE

**Core functions:**

| Function | Purpose | Method |
|----------|---------|--------|
| `extract_manifest()` | Read 3 TS files → ExtractedManifest | File I/O + parsing |
| `extract_commands()` | Source → CommandRegistry (131 entries) | Line-by-line import parsing + feature gate detection |
| `extract_tools()` | Source → ToolRegistry (31 entries) | `import` + `ends_with("Tool")` |
| `extract_bootstrap_plan()` | CLI source → BootstrapPlan (12 phases) | String literal detection |

**Helpers:**
- `imported_symbols()` — Parse `import { A, B, C }`
- `first_identifier()` — Extract alphanumeric sequences
- `dedupe_commands()` / `dedupe_tools()` — Remove duplicates

**Tests (3 passing):**
1. `extracts_non_empty_manifests_from_upstream_repo()` — Validates ≥1 command, tool, phase
2. `detects_known_upstream_command_symbols()` — Checks for `addDir`, `review`
3. `detects_known_upstream_tool_symbols()` — Checks for `AgentTool`, `BashTool`

---

## 3. TypeScript Source Scope (src/)

- **1,884 files** | **512,664 lines**
- Full Anthropic Claude Code CLI implementation (Bun runtime)

### Largest Directories

| Directory | Size | Files | Purpose |
|-----------|------|-------|---------|
| src/components/ | 10M | ~140 | React/Ink UI |
| src/commands/ | 3.0M | ~85 | Slash commands |
| src/hooks/ | 1.5M | — | React hooks |
| src/ink/ | 1.3M | — | React → ANSI renderer |
| src/bridge/ | 536K | — | IDE integration |
| src/cli/ | 528K | — | CLI output & I/O |

### Key Files by LOC

| File | LOC | Purpose |
|------|-----|---------|
| src/cli/print.ts | 5,594 | Output formatting |
| src/utils/messages.ts | 5,512 | Message composition |
| src/utils/sessionStorage.ts | 5,105 | Session persistence |
| src/screens/REPL.tsx | 5,005 | Interactive REPL |
| src/main.tsx | 4,683 | CLI entry point |
| src/utils/bash/bashParser.ts | 4,436 | Bash syntax parsing |
| src/services/api/claude.ts | 3,419 | Anthropic API client |
| src/services/mcp/client.ts | 3,348 | MCP client |

### Extracted Manifests (by Rust compat-harness)

- **131 commands** (e.g., /commit, /review, /doctor, /mcp, /config)
- **31 tools** (e.g., BashTool, FileReadTool, GrepTool, AgentTool)
- **12 bootstrap phases**

---

## 4. Porting Status Assessment

### What's IMPLEMENTED

- Manifest extraction (131 commands, 31 tools, 12 phases)
- Bootstrap phase definitions (12 phases as enum)
- Registry types (CommandRegistry, ToolRegistry)
- Test infrastructure (3 integration tests passing)
- Source parsing (imports, feature gates, CLI fast paths)

### What's NOT IMPLEMENTED

- CLI argument parsing (no clap yet)
- Tool execution (BashTool, FileReadTool, GrepTool, etc.)
- Command execution (131 commands detected but 0 implemented)
- State management (config, sessions, history)
- API integration (no Anthropic SDK, no streaming)
- Terminal UI (no crossterm/ratatui)
- MCP, IDE bridge, plugins, agents, worktrees
- Permission system
- OAuth/auth
- Feature flags
- Telemetry

### Completion by Feature

| Feature | Status | % Done |
|---------|--------|--------|
| Manifest extraction | Complete | 100% |
| CLI scaffolding | Stub only | 5% |
| Bootstrap dispatch | Planned | 0% |
| File operations | Not started | 0% |
| Shell execution | Not started | 0% |
| API integration | Not started | 0% |
| Terminal UI | Not started | 0% |
| **Overall** | Proof phase | **~7%** |

### Dependencies NOT Yet Added

- **tokio** — Async runtime
- **clap** — CLI parsing
- **serde + serde_json** — Configuration
- **crossterm/ratatui** — Terminal UI
- **reqwest** — HTTP client
- **anyhow + thiserror** — Error handling
- **tracing** — Logging
- **regex** — Pattern matching

---

## 5. Design Philosophy

From Rust README:

> "Harness-first scaffolding, not full feature parity"

**Approach:** Extract facts from upstream source before implementing execution. Low risk, verifiable.

**Constraints:**
- Initial delivery stays in `rust/` subdirectory
- Workspace started empty, needed proof-oriented scaffolding
- Rejected: "Feature-complete CLI rewrite" (too risky without harness)

---

## 6. Git History

| Commit | Message |
|--------|---------|
| d621f5d | **Establish a harness-first Rust port foundation** (main Rust work) |
| c941e95 | docs: emphasize leaked source |
| c9f7b96 | docs: rewrite README in English |
| 66d9c1e | docs: add comprehensive README |
| caad050 | init: add source code from src.zip |
| 01bf54a | Rewriting Project Claw Code - Python port |
| 507c246 | Make repository's primary source tree genuinely Python |

All commits from 2026-03-31. Single day of work.

---

## 7. Legal Article Summary

`2026-03-09-is-legal-the-same-as-legitimate-ai-reimplementation-and-the-erosion-of-copyleft.md`

Author: Hong Minhee (~5,800 words). Thesis: "Legal ≠ Legitimate" — clearing the legal bar for AI-generated reimplementation doesn't make it right. Uses the chardet (LGPL→MIT via Claude) case study. Archived here as ethical reflection on the project's nature.

---

## 8. Inferred Roadmap

- **Phase 0 (Current):** Manifest extraction & scaffolding
- **Phase 1:** CLI & dispatch logic (clap, bootstrap executor)
- **Phase 2:** Tool execution (file ops, bash, permissions)
- **Phase 3:** API integration (Anthropic SDK, streaming)
- **Phase 4:** Full feature parity (131 commands, 31 tools, bridge, plugins, MCP)

---

## 9. Key Takeaway for Our Rewrite

The dev/rust branch is **barely started** — 481 lines of Rust doing manifest extraction only. The valuable parts are:

1. **The 12 bootstrap phases** — useful architecture reference
2. **The compat-harness pattern** — extracting facts from upstream source
3. **The TypeScript source** — the complete reference implementation (512K LOC)
4. **The philosophy** — "harness-first, not feature-complete rewrite"

Our rewrite can use RTK's patterns (clap, anyhow, rusqlite, ratatui) and ECC2's session management as much more mature Rust foundations than what's here.
