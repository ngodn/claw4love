# Claw-Code: Python Porting Project Analysis

## Executive Summary

**Claw-Code** is a Python rewrite of Claude Code CLI, created as a clean-room port. The project is in active development with a **Rust port in progress** on the `dev/rust` branch. The Python workspace serves as:
- A functional harness runtime with complete command/tool routing
- A reference implementation for porting patterns
- A mirrored snapshot system maintaining parity with the original TypeScript architecture

**Current Status:** 67 Python files across 40+ modules, mirroring 150+ commands and 100+ tools from the archived snapshot.

---

## 1. Project Structure

```
claw-code/
├── .github/
│   └── FUNDING.yml
├── assets/
│   ├── clawd-hero.jpeg
│   ├── instructkr.png
│   ├── omx/                                 # OmX workflow orchestration screenshots
│   ├── star-history.png
│   ├── tweet-screenshot.png
│   └── wsj-feature.png
├── src/                                     # PRIMARY: Python porting workspace
│   ├── __init__.py                          # Package export surface
│   ├── main.py                              # CLI entrypoint with 25+ subcommands
│   ├── models.py                            # Core dataclasses
│   ├── commands.py                          # Command registry (150+ entries)
│   ├── tools.py                             # Tool registry (100+ entries)
│   ├── query_engine.py                      # Session/turn management
│   ├── QueryEngine.py                       # Runtime class variant
│   ├── runtime.py                           # Orchestration and routing
│   ├── context.py                           # Workspace context building
│   ├── setup.py                             # Initialization
│   ├── system_init.py                       # System init message builder
│   ├── port_manifest.py                     # Workspace metadata
│   ├── parity_audit.py                      # Version tracking
│   ├── permissions.py                       # Access control
│   ├── Tool.py                              # Tool metadata
│   ├── task.py                              # Task planning
│   ├── tasks.py                             # Task defaults
│   ├── history.py                           # Session tracking
│   ├── transcript.py                        # Message persistence
│   ├── session_store.py                     # Session serialization
│   ├── execution_registry.py                # Command/tool execution
│   ├── bootstrap_graph.py                   # Startup sequence
│   ├── command_graph.py                     # Command segmentation
│   ├── tool_pool.py                         # Tool assembly
│   ├── remote_runtime.py                    # Remote execution modes
│   ├── direct_modes.py                      # Direct connectivity
│   ├── cost_tracker.py                      # Token accounting
│   ├── costHook.py                          # Cost instrumentation
│   ├── deferred_init.py                     # Delayed initialization
│   ├── prefetch.py                          # Async startup
│   ├── ink.py                               # Formatting
│   ├── interactiveHelpers.py                # Text utilities
│   ├── dialogLaunchers.py                   # UI entry points
│   ├── query.py                             # Message DTOs
│   ├── replLauncher.py                      # REPL messaging
│   ├── projectOnboardingState.py            # Onboarding tracking
│   ├── reference_data/
│   │   ├── __init__.py
│   │   ├── archive_surface_snapshot.json    # Root files & dirs from TS archive
│   │   ├── commands_snapshot.json           # 1036 lines, 150+ commands
│   │   ├── tools_snapshot.json              # 921 lines, 100+ tools
│   │   └── subsystems/                      # 24 subsystem metadata JSONs
│   │       ├── assistant.json ... voice.json
│   └── [24 subsystem packages]/             # Each has __init__.py loading JSON
│       └── __init__.py
├── tests/
│   └── test_porting_workspace.py            # 249 lines, 26 test methods
├── .gitignore
└── README.md
```

### Module Counts
- **Core operational:** 15 (query_engine, runtime, context, setup, etc.)
- **Registry/snapshot:** 3 (commands, tools, execution_registry)
- **Data structures:** 8 (models, task, history, transcript, session_store, etc.)
- **UI/formatting:** 4 (ink, interactiveHelpers, dialogLaunchers, replLauncher)
- **Subsystem packages:** 24 (skeleton `__init__.py` with JSON metadata)
- **Utility:** 7 (cost_tracker, costHook, deferred_init, prefetch, etc.)
- **Runtime modes:** 2 (remote_runtime, direct_modes)
- **Graphs/pools:** 3 (bootstrap_graph, command_graph, tool_pool)

---

## 2. Entry Points and Execution Flow

### Primary Entry Point: `src/main.py` (214 lines)

- `build_parser()` → creates ArgumentParser with 25+ subcommands
- `main(argv)` → main CLI dispatcher (lines 94-209)

### CLI Subcommands (25 total)

| Command | Handler | Purpose |
|---------|---------|---------|
| `summary` | QueryEnginePort.render_summary() | Render workspace summary |
| `manifest` | PortManifest.to_markdown() | Print file manifest |
| `parity-audit` | run_parity_audit() | Compare against TS archive |
| `setup-report` | run_setup() | Show init status |
| `command-graph` | build_command_graph() | Display command segmentation |
| `tool-pool` | assemble_tool_pool() | Show assembled tools |
| `bootstrap-graph` | build_bootstrap_graph() | Show startup stages |
| `subsystems` | List top-level modules | Enumerate modules |
| `commands` | get_commands() | List mirrored commands |
| `tools` | get_tools() | List mirrored tools |
| `route` | PortRuntime.route_prompt() | Match prompt to commands/tools |
| `bootstrap` | PortRuntime.bootstrap_session() | Build session report |
| `turn-loop` | PortRuntime.run_turn_loop() | Run multi-turn loop |
| `flush-transcript` | QueryEnginePort.persist_session() | Persist transcript |
| `load-session` | load_session() | Restore session |
| `remote-mode` | run_remote_mode() | Remote branching |
| `ssh-mode` | run_ssh_mode() | SSH branching |
| `teleport-mode` | run_teleport_mode() | Teleport branching |
| `direct-connect-mode` | run_direct_connect() | Direct-connect branching |
| `deep-link-mode` | run_deep_link() | Deep-link branching |
| `show-command` | get_command() | Show one command |
| `show-tool` | get_tool() | Show one tool |
| `exec-command` | execute_command() | Execute command shim |
| `exec-tool` | execute_tool() | Execute tool shim |

### Execution Flow

```
main.py → build_parser() → parse_args
  → build_port_manifest()
  → dispatch on args.command:
    ├── 'summary' → QueryEnginePort(manifest).render_summary()
    ├── 'bootstrap' → PortRuntime().bootstrap_session()
    ├── 'route' → PortRuntime().route_prompt()
    ├── 'turn-loop' → PortRuntime().run_turn_loop()
    └── ... (21 more)
  → return 0 or 1
```

---

## 3. Core Modules

### 3.1 Models (`src/models.py`, 50 lines)

| Class | Fields | Purpose |
|-------|--------|---------|
| `Subsystem` | name, path, file_count, notes | Python package in workspace |
| `PortingModule` | name, responsibility, source_hint, status | Command/tool entry |
| `PermissionDenial` | tool_name, reason | Denied tool access |
| `UsageSummary` | input_tokens, output_tokens | Token tracking |
| `PortingBacklog` | title, modules[] | Module aggregation |

### 3.2 Command Registry (`src/commands.py`, 91 lines)

- **Snapshot:** `reference_data/commands_snapshot.json` (1036 lines, 150+ entries)
- `load_command_snapshot()` → cached tuple of PortingModule
- `get_command(name)` → case-insensitive lookup
- `get_commands(cwd, include_plugin, include_skill)` → filtered list
- `find_commands(query, limit)` → text search
- `execute_command(name, prompt)` → shim executor
- **Global:** `PORTED_COMMANDS` cached tuple

### 3.3 Tool Registry (`src/tools.py`, 97 lines)

- **Snapshot:** `reference_data/tools_snapshot.json` (921 lines, 100+ entries)
- `load_tool_snapshot()` → cached tuple
- `get_tool(name)` → case-insensitive lookup
- `filter_tools_by_permission_context()` → access control
- `get_tools(simple_mode, include_mcp, permission_context)` → filtered
- **Global:** `PORTED_TOOLS` cached tuple
- **Filters:** simple_mode (bash/read/edit only), include_mcp, permission_context blocklist

### 3.4 Query Engine (`src/query_engine.py`, 194 lines)

**Config:** `QueryEngineConfig` - max_turns=8, max_budget=2000, compact_after=12

**Class:** `QueryEnginePort`
- `from_workspace()` / `from_saved_session(session_id)` - constructors
- `submit_message(prompt, commands, tools, denied)` → TurnResult
- `stream_submit_message(...)` → generator yielding events
- `persist_session()` → flush transcript + save to disk
- `render_summary()` → markdown

**TurnResult fields:** prompt, output, matched_commands, matched_tools, permission_denials, usage, stop_reason

### 3.5 Runtime (`src/runtime.py`, 193 lines)

**Class:** `PortRuntime`
- `route_prompt(prompt, limit=5)` → list[RoutedMatch]
  - Tokenizes on whitespace/slash/dash, scores against commands+tools
- `bootstrap_session(prompt, limit=5)` → RuntimeSession
  - Full session: context → setup → route → execute → stream → persist
- `run_turn_loop(prompt, limit, max_turns, structured_output)` → list[TurnResult]

### 3.6 Other Core Modules

| Module | Lines | Key Exports |
|--------|-------|-------------|
| `context.py` | 48 | PortContext, build_port_context() |
| `setup.py` | 78 | WorkspaceSetup, SetupReport, run_setup() |
| `system_init.py` | 24 | build_system_init_message() |
| `port_manifest.py` | 53 | PortManifest, build_port_manifest() |
| `parity_audit.py` | 139 | ParityAuditResult, run_parity_audit() |
| `permissions.py` | 21 | ToolPermissionContext (frozen, deny_names/prefixes) |
| `session_store.py` | 36 | StoredSession, save_session(), load_session() |
| `execution_registry.py` | 52 | ExecutionRegistry, MirroredCommand, MirroredTool |
| `bootstrap_graph.py` | 28 | BootstrapGraph (7-stage startup) |
| `command_graph.py` | 35 | CommandGraph (builtins/plugin/skill categories) |
| `tool_pool.py` | 38 | ToolPool, assemble_tool_pool() |

---

## 4. Key Data Structures

### Frozen Dataclasses (Immutable)
```
Subsystem, PortingModule, PermissionDenial, UsageSummary, PortingBacklog
QueryEngineConfig, TurnResult, RoutedMatch
StoredSession, HistoryEvent
ToolPermissionContext, BootstrapGraph
RuntimeModeReport, DirectModeReport
PrefetchResult, DeferredInitResult
```

### Mutable Classes
```
PortManifest, QueryEnginePort, HistoryLog, TranscriptStore
CostTracker, RuntimeSession
```

---

## 5. Dependencies

**Zero external packages.** Pure Python stdlib only:
- argparse, collections, dataclasses, functools, json, pathlib
- platform, subprocess, sys, unittest, uuid

---

## 6. Subsystem Packages (24 total)

All follow identical pattern - `src/{name}/__init__.py` (17 lines each):
- Load JSON from `reference_data/subsystems/{name}.json`
- Export: ARCHIVE_NAME, MODULE_COUNT, SAMPLE_FILES, PORTING_NOTE

**Subsystems:** assistant, bootstrap, bridge, buddy, cli, components, constants, coordinator, entrypoints, hooks, keybindings, memdir, migrations, moreright, native_ts, outputStyles, plugins, remote, schemas, screens, server, services, skills, state, types, upstreamproxy, utils, vim, voice

---

## 7. Tests (`tests/test_porting_workspace.py`, 249 lines)

26 test methods (17 integration, 9 unit) covering all 25 CLI commands.

---

## 8. Architectural Patterns

1. **Snapshot-Based Mirroring** - Commands/tools loaded from JSON at module level with `@lru_cache`
2. **Session Manager Pattern** - QueryEnginePort manages mutable state, most data frozen
3. **Token-Based Routing** - Simple substring matching for prompt→command/tool routing
4. **Permission Blocklist** - frozenset/tuple deny lists for tool access control
5. **Subsystem Placeholders** - Skeleton packages with JSON metadata for gradual porting
6. **Immutable Data Everywhere** - Heavy `@dataclass(frozen=True)` usage

---

## 9. Rust Rewrite Notes

- All frozen dataclasses → Rust structs trivially
- CLI dispatcher → clap crate
- JSON snapshots → serde_json
- Permission system → HashSet/BTreeSet
- No concurrency primitives → straightforward async migration
- Branches: `dev/rust` (remote-only) has Rust port in progress
