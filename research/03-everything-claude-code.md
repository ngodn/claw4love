# Everything Claude Code (ECC): Optimization Plugin Analysis

## Executive Summary

**Everything Claude Code (ECC)** (v1.9.0) is a production-ready AI coding plugin/harness system by Anthropic Hackathon winner Affaan Mustafa. It provides battle-tested configurations, agents, skills, hooks, commands, rules, and MCP integrations for Claude Code CLI and compatible platforms (Cursor, Codex, OpenCode, Antigravity IDE).

**Scale:**
- 30 specialized agents
- 136 skills (workflow definitions)
- 60 commands (slash-command library)
- 29 hooks (event-driven automations)
- 13 language-specific rule sets + common rules
- 6 install profiles (core, developer, security, research, full, custom)
- 12 language ecosystems: TypeScript, Python, Go, Java, Kotlin, Rust, Swift, PHP, Perl, C++, C#
- 50K+ GitHub stars, MIT licensed

---

## 1. Project Structure

```
everything-claude-code/
├── agents/                    # 30 specialized subagent definitions (Markdown+YAML)
├── skills/                    # 136 skills across 10+ domains
├── commands/                  # 60 slash-commands (Markdown+YAML)
├── hooks/                     # 29 event-driven automations
│   ├── hooks.json            # Hook definitions
│   └── README.md
├── rules/                     # Language-specific coding standards
│   ├── common/               # Universal rules (8 files)
│   ├── cpp/ csharp/ golang/ java/ kotlin/ perl/ php/
│   ├── python/ rust/ swift/ typescript/
│   └── zh/                   # Chinese language rules
├── contexts/                  # Session context guidance (dev, research, review)
├── manifests/                 # Selective install system
│   ├── install-modules.json  # 20+ modules
│   ├── install-profiles.json # 5 profiles
│   └── install-components.json
├── schemas/                   # JSON Schema definitions
├── scripts/                   # Node.js CLI utilities
│   ├── ci/                   # CI validation (8 validators)
│   ├── hooks/                # Hook implementations
│   ├── lib/
│   │   ├── install/          # Install engine (apply, config, request, runtime)
│   │   ├── install-targets/  # Target adapters (claude, cursor, codex, etc.)
│   │   ├── session-adapters/ # Cross-harness session recording
│   │   ├── skill-evolution/  # Continuous learning (tracker, versioning, health)
│   │   ├── skill-improvement/# Skill self-improvement
│   │   └── state-store/      # SQLite state management
│   ├── ecc.js                # Unified CLI entrypoint
│   ├── install-apply.js      # Installer
│   ├── harness-audit.js      # Performance audit
│   ├── doctor.js             # Health check
│   └── [20+ more scripts]
├── ecc2/                      # ECC 2.0 Rust TUI control plane
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # CLI (dashboard, start, sessions, status, stop, resume, daemon)
│       ├── session/          # Session management (mod, daemon, manager, output, runtime, store)
│       ├── tui/              # Ratatui dashboard (app, dashboard, widgets)
│       ├── worktree/         # Git worktree management
│       ├── comms/            # IPC
│       ├── config/           # Configuration
│       └── observability/    # Logging/tracing
├── .agents/ .claude/ .codex/ .cursor/ .kiro/ .opencode/  # Platform-specific configs
├── .claude-plugin/ .codex-plugin/  # Plugin metadata
├── mcp-configs/              # MCP server configurations
├── plugins/                   # Third-party integrations
├── docs/                      # Documentation (72 files, 6 translations)
├── tests/                     # Test suite
├── examples/                  # Example configs
├── CLAUDE.md / AGENTS.md / SOUL.md  # Core guidance
├── the-shortform-guide.md / the-longform-guide.md / the-security-guide.md
├── package.json              # ecc-universal v1.9.0
└── VERSION                   # 1.9.0
```

---

## 2. Core Identity & Principles (SOUL.md)

Five Core Principles:
1. **Agent-First** — Route work to the right specialist early
2. **Test-Driven** — Write/refresh tests before trusting changes
3. **Security-First** — Validate inputs, protect secrets, safe defaults
4. **Immutability** — Prefer explicit state transitions over mutation
5. **Plan Before Execute** — Complex changes broken into deliberate phases

---

## 3. Agents (30 total, `agents/`)

Each agent is a Markdown file with YAML frontmatter (name, description, tools, model).

### Agent Categories

**Planning & Architecture:**
| Agent | File | Size | Purpose |
|-------|------|------|---------|
| planner | agents/planner.md | 213 lines | Feature implementation planning |
| architect | agents/architect.md | 6.1K | System design decisions |

**Code Review (language-specific):**
| Agent | File | Size | Purpose |
|-------|------|------|---------|
| code-reviewer | agents/code-reviewer.md | 8.6K | General code quality |
| typescript-reviewer | agents/typescript-reviewer.md | 7.5K | TS/JS review |
| python-reviewer | agents/python-reviewer.md | 3.3K | Python review |
| rust-reviewer | agents/rust-reviewer.md | 4.6K | Rust review |
| go-reviewer | agents/go-reviewer.md | 2.7K | Go review |
| java-reviewer | agents/java-reviewer.md | 5.6K | Java/Spring review |
| kotlin-reviewer | agents/kotlin-reviewer.md | 6.5K | Kotlin/KMP review |
| cpp-reviewer | agents/cpp-reviewer.md | 2.9K | C++ review |
| flutter-reviewer | agents/flutter-reviewer.md | 13.9K | Flutter/Dart review |
| database-reviewer | agents/database-reviewer.md | 4.2K | PostgreSQL/Supabase |

**Build Error Resolution:**
| Agent | File | Size | Purpose |
|-------|------|------|---------|
| build-error-resolver | agents/build-error-resolver.md | 3.7K | General build errors |
| cpp-build-resolver | agents/cpp-build-resolver.md | 3.1K | C++ builds |
| go-build-resolver | agents/go-build-resolver.md | 3.2K | Go builds |
| java-build-resolver | agents/java-build-resolver.md | 5.6K | Java/Maven/Gradle |
| kotlin-build-resolver | agents/kotlin-build-resolver.md | 4.1K | Kotlin builds |
| rust-build-resolver | agents/rust-build-resolver.md | 5.7K | Rust builds |
| pytorch-build-resolver | agents/pytorch-build-resolver.md | 5.4K | PyTorch/CUDA |

**Workflow & Utility:**
| Agent | File | Purpose |
|-------|------|---------|
| tdd-guide | agents/tdd-guide.md | Test-driven development |
| security-reviewer | agents/security-reviewer.md | Security analysis |
| e2e-runner | agents/e2e-runner.md | Playwright E2E testing |
| performance-optimizer | agents/performance-optimizer.md | Performance analysis |
| refactor-cleaner | agents/refactor-cleaner.md | Dead code cleanup |
| doc-updater | agents/doc-updater.md | Documentation updates |
| docs-lookup | agents/docs-lookup.md | API/library research |
| chief-of-staff | agents/chief-of-staff.md | Communication triage |
| harness-optimizer | agents/harness-optimizer.md | Config tuning |
| loop-operator | agents/loop-operator.md | Autonomous loops |
| healthcare-reviewer | agents/healthcare-reviewer.md | PHI compliance |

---

## 4. Skills (136 total, `skills/`)

### Skill Categories

**Language/Framework Patterns (60+ skills):**
- TypeScript: bun-runtime, nextjs-turbopack
- Python: django-* (4), pytorch-patterns, python-* (2)
- Go: golang-patterns, golang-testing
- Java: springboot-* (4), java-coding-standards, jpa-patterns
- Kotlin: kotlin-* (4)
- Rust: rust-patterns, rust-testing
- Swift: swift-* (3), swiftui-patterns
- PHP: laravel-* (5)
- Perl: perl-* (3)
- C++: cpp-coding-standards, cpp-testing

**Workflow & Process (40+ skills):**
- Development: tdd-workflow, verification-loop, code-review, coding-standards
- Planning: architecture-decision-records, blueprint, strategic-compact
- AI/LLM: ai-first-engineering, agentic-engineering, cost-aware-llm-pipeline, prompt-optimizer
- Research: deep-research, documentation-lookup, market-research, exa-search
- Content: article-writing, content-engine, video-editing
- DevOps: ci-patterns, kubernetes-patterns, deployment-patterns, docker-patterns

**Domain-Specific (30+ skills):**
- Healthcare: healthcare-cdss, healthcare-emr, healthcare-phi-compliance
- Finance/Supply Chain: energy-procurement, inventory-demand-planning, production-scheduling
- Business: investor-materials, investor-outreach, team-builder, product-lens

**Advanced/Emerging (15+ skills):**
- continuous-learning-v2 (hooks + scripts for skill self-improvement)
- autonomous-loops, agentic-engineering, agent-harness-construction

### Skill Format
```
skills/<skill-name>/
├── README.md              # When to use, how it works, examples
├── agent.yaml             # Optional: skill-specific subagent config
├── codemaps/              # Optional: code navigation shortcuts
├── hooks/                 # Optional: lifecycle automations
└── scripts/               # Optional: utility scripts
```

---

## 5. Commands (60 total, `commands/`)

Each command is a Markdown file with `description:` frontmatter.

**Key Commands:**
- `/tdd` — Test-driven development workflow
- `/plan` — Feature implementation planning
- `/e2e` — Playwright end-to-end testing
- `/code-review` — Code quality review
- `/build-fix` — Build error resolution
- `/security-review` — Security audit
- `/deep-research` — Research synthesis
- `/devfleet` — Multi-agent orchestration
- `/loop-start` — Autonomous loop execution
- `/harness-audit` — Harness performance audit
- `/evolve` — Pattern extraction from sessions
- `/skills-health` — Skill quality assessment

---

## 6. Hooks (29 total, `hooks/hooks.json`)

### PreToolUse Hooks (can block with exit code 2)

| Hook | Matcher | Purpose |
|------|---------|---------|
| block-no-verify | Bash | Block `--no-verify` git flag |
| auto-tmux-dev | Bash | Auto-start dev servers in tmux |
| tmux-reminder | Bash | Suggest tmux for long commands |
| git-push-reminder | Bash | Review before push |
| pre-commit-quality | Bash | Lint, commit format, secrets |
| doc-file-warning | Write | Warn about non-standard docs |
| suggest-compact | Edit/Write | Suggest manual /compact |
| observe | * | Capture tool observations (async, continuous learning) |

### PostToolUse Hooks

| Hook | Matcher | Purpose |
|------|---------|---------|
| pr-logger | Bash | Log PR URLs |
| build-analysis | Bash | Background build analysis |
| quality-gate | Edit/Write | Fast quality checks |
| prettier-format | Edit | Auto-format JS/TS |
| typescript-check | Edit | Run tsc on .ts/.tsx edits |
| console-log-warning | Edit | Warn about console.log |

### Lifecycle Hooks

| Hook | Event | Purpose |
|------|-------|---------|
| session-start | SessionStart | Load context from previous sessions |
| pre-compact | PreCompact | Save state before compaction |
| console-audit | Stop | Check for console.log in modified files |
| session-summary | Stop | Persist session state |
| pattern-extraction | Stop | Evaluate for extractable patterns |
| cost-tracker | Stop | Emit cost telemetry |
| desktop-notify | Stop | macOS notification |

### Hook Control

```bash
export ECC_HOOK_PROFILE=minimal|standard|strict  # default: standard
export ECC_DISABLED_HOOKS="pre:bash:tmux-reminder,post:edit:typecheck"
```

---

## 7. Rules (13 languages + common, `rules/`)

```
rules/
├── common/           # Universal (coding-style, git-workflow, testing, performance,
│                     #            patterns, security, hooks, agents)
├── cpp/ csharp/ golang/ java/ kotlin/ perl/ php/
├── python/ rust/ swift/ typescript/
└── zh/               # Chinese language rules
```

Language-specific rules **override** common rules (CSS specificity pattern).

---

## 8. Installation System (Manifest-Driven)

### Install Profiles (`manifests/install-profiles.json`)

| Profile | Use Case |
|---------|----------|
| core | Minimal harness baseline |
| developer | Default for app development |
| security | Security-focused development |
| research | Research and content workflows |
| full | Complete ECC install |

### Install Targets

| Target | Root | Scope |
|--------|------|-------|
| claude (default) | ~/.claude/ | User home |
| cursor | ./.cursor/ | Project |
| codex | ~/.codex/ | User home |
| antigravity | ./.agent/ | Project |
| opencode | ~/.opencode/ | User home |

### Install Flow
1. Parse request (languages, --profile, --modules, --with/--without)
2. Load manifests, resolve dependencies
3. Create install plan (all file operations)
4. Apply plan (copy, merge, record)
5. Write install-state.json (for uninstall/repair)

---

## 9. ECC 2.0 Rust TUI Control Plane (`ecc2/`)

Emerging control plane for managing Claude Code sessions.

### Dependencies
- TUI: ratatui 0.29 + crossterm 0.28
- Async: tokio
- State: rusqlite 0.32 (SQLite)
- Git: git2 0.20
- CLI: clap 4
- Serialization: serde, serde_json, toml
- Logging: tracing

### Source Structure

| File | Purpose |
|------|---------|
| main.rs (127 lines) | CLI: dashboard, start, sessions, status, stop, resume, daemon |
| session/mod.rs (103 lines) | Session struct, SessionState enum (Pending→Running→Idle/Completed/Failed→Stopped) |
| session/store.rs | SQLite-backed state persistence |
| session/manager.rs | Session lifecycle CRUD |
| session/daemon.rs | Background daemon |
| session/runtime.rs | Execution runtime |
| tui/app.rs | TUI event loop |
| tui/dashboard.rs | Dashboard rendering |
| tui/widgets.rs | Ratatui widgets |
| worktree/mod.rs | Git worktree operations |

### Key Design
Sessions are first-class entities with durable state, metrics, and lifecycle. Enables:
- Background agent execution
- Multi-session orchestration
- Deterministic replay
- Cost and performance tracking

---

## 10. Key Architectural Patterns

### 10.1 Manifest-Driven Installation
- Declarative modules, profiles, components
- Deterministic, replayable, reversible
- install-state.json tracks all operations

### 10.2 Session Adapters (`scripts/lib/session-adapters/`)
- Adapter pattern for cross-harness compatibility
- canonical-session.js (universal schema), claude-history.js, dmux-tmux.js

### 10.3 Continuous Learning Pipeline (`continuous-learning-v2/`)
- observe.sh (PreToolUse hook) records tool use
- evaluate.js identifies extractable patterns
- amendify.js updates skills with new patterns
- tracker.js versions and tracks provenance
- health.js measures pattern utility

### 10.4 State Store (`scripts/lib/state-store/`)
- SQLite-backed: sessions, tool_calls, learned_patterns tables
- Schema migrations, prepared queries
- CLI interface for querying

### 10.5 Orchestration (`tmux-worktree-orchestrator.js`)
- tmux session per agent
- git worktree per session (isolated branch)
- Log routing to named windows
- Cleanup on session end

---

## 11. MCP Configuration (`.mcp.json`)

| Server | Purpose |
|--------|---------|
| GitHub | PR/issue management |
| Context7 | Distributed context caching |
| Exa | Semantic search |
| Memory | Persistent session memory |
| Playwright | Browser automation |
| Sequential Thinking | Extended reasoning |

---

## 12. Testing & CI

**CI Pipeline (8 validators):**
- check-unicode-safety.js
- validate-agents.js, validate-commands.js, validate-rules.js
- validate-skills.js, validate-hooks.js
- validate-install-manifests.js, validate-no-personal-paths.js

**Coverage:** 80%+ required (lines, functions, branches, statements)

---

## 13. Documentation

| Guide | Lines | Topic |
|-------|-------|-------|
| the-shortform-guide.md | 431 | Quick setup, skills, hooks, agents |
| the-longform-guide.md | 354 | Token optimization, memory, evals, parallelization |
| the-security-guide.md | — | Attack vectors, sandboxing, sanitization |
| SOUL.md | 17 | Core identity and principles |

**Technical Docs:** SELECTIVE-INSTALL-ARCHITECTURE.md, SESSION-ADAPTER-CONTRACT.md, SKILL-DEVELOPMENT-GUIDE.md, ECC-2.0-REFERENCE-ARCHITECTURE.md

**Translations:** pt-BR, zh-CN, zh-TW, ja-JP, ko-KR, tr

---

## 14. Integration with Claude Code CLI

Five integration mechanisms:
1. **Rules** (`~/.claude/rules/`) — Always-follow coding standards, auto-loaded by language
2. **Skills** (`~/.claude/skills/`) — Contextual workflow guidance, loaded on demand
3. **Agents** (`~/.claude/agents/`) — Specialized subprocesses with constrained tools
4. **Hooks** (`~/.claude/hooks.json`) — Event-driven automations (pre/post tool, lifecycle)
5. **Commands** (`~/.claude/commands/`) — Slash-command entry points to workflows
