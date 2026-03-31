# Leak-Claude-Code: Claude Code CLI Source Analysis

## Executive Summary

The Claude Code project is Anthropic's official terminal-based CLI for AI-assisted coding, built as a **single-binary application** using TypeScript, React+Ink for terminal UI, and Bun runtime. The source contains **~2,172 files** and **512,000+ lines of TypeScript**:

- **Core Query Engine** (~46K lines) handling LLM streaming, tool-calling loops, thinking mode
- **~40 specialized tools** for file ops, shell execution, web access, MCP integration
- **~85 slash commands** for user interaction
- **React-based terminal UI** with ~140 components
- **Advanced subsystems** for permissions, plugins, skills, memory, bridge/IDE integration
- **Multi-agent support** via coordinator, async agents, and team systems

---

## 1. Project Structure

### Root Level

```
leak-claude-code/
├── package.json                    # Bun package manager config
├── tsconfig.json                   # TypeScript strict mode
├── biome.json                      # Biome formatter/linter
├── bun.lock / bunfig.toml          # Bun runtime config
├── .env.example                    # Environment template
├── .mcp.json                       # MCP server config
├── Dockerfile / docker/            # Container build
├── LICENSE                         # UNLICENSED (proprietary)
├── README.md / CONTRIBUTING.md     # Docs
├── Skill.md / agent.md             # Skills & agent framework docs
├── docs/                           # Architecture documentation
│   ├── architecture.md
│   ├── subsystems.md
│   ├── tools.md
│   ├── commands.md
│   ├── exploration-guide.md
│   └── bridge.md
├── prompts/                        # 16 sequential build-out prompts
│   ├── 00-overview.md ... 16-testing.md
├── scripts/                        # Build & dev scripts
│   ├── build-bundle.ts             # ESBuild bundler
│   ├── build-web.ts / dev.ts       # Web & dev runners
│   ├── bun-plugin-shims.ts         # Bun polyfills
│   └── test-*.ts / package-npm.ts  # Testing & packaging
├── mcp-server/                     # Standalone MCP server
│   └── src/ (index.ts, http.ts, server.ts, vercelApp.ts)
├── web/                            # Web UI
└── src/                            # **MAIN APPLICATION SOURCE**
```

### src/ Directory Overview

```
src/
├── entrypoints/          # Application entry points (CLI, MCP, SDK)
├── assistant/            # Assistant & session history
├── bootstrap/            # Initialization & startup
├── bridge/               # IDE bridge integration (~16 files)
├── buddy/                # Companion system
├── cli/                  # CLI infrastructure & transports
├── commands/             # ~85 slash commands
├── components/           # ~140 React/Ink terminal components
├── context/              # React context providers
├── coordinator/          # Multi-agent orchestration
├── hooks/                # ~80 React hooks
├── memdir/               # Memory system (CLAUDE.md)
├── migrations/           # Config migration system
├── plugins/              # Plugin system
├── screens/              # Full-screen UI modes (REPL, Doctor, Resume)
├── schemas/              # Zod validation schemas
├── services/             # Service layer (API, analytics, MCP, plugins, skills)
├── shims/                # Bun & Node compatibility
├── skills/               # Skill system (16 bundled)
├── state/                # State management (AppStateStore)
├── tasks/                # Background task management
├── tools/                # ~40 agent tools
├── types/                # TypeScript type definitions
├── utils/                # Utility functions & helpers
├── vim/                  # Vim mode implementation
├── voice/                # Voice input/output
├── QueryEngine.ts        # **HEART** (~46K lines)
├── Tool.ts               # Tool interface & registry (~795 lines)
├── Task.ts               # Task type definitions
├── commands.ts           # Command registry (~400+ lines)
├── tools.ts              # Tool registry (~400+ lines)
├── cost-tracker.ts       # Token usage & cost tracking
├── query.ts              # Query entry point
├── replLauncher.tsx      # REPL launcher
└── main.tsx              # Main entrypoint (Commander.js CLI)
```

---

## 2. Entry Points & Main Execution Flow

### 2.1 CLI Entry: `src/main.tsx`
- Commander.js CLI argument parsing
- Prefetch side-effects (MDM, keychain, API preconnect)
- Initialize React/Ink renderer → REPL launcher

### 2.2 Initialization: `src/entrypoints/`

| File | Role |
|------|------|
| `cli.tsx` | Main CLI session orchestration (~1,500 lines) |
| `init.ts` | Config, telemetry, OAuth, MDM policy setup (~800 lines) |
| `mcp.ts` | MCP server mode (~400 lines) |
| `agentSdkTypes.ts` | Agent SDK types (~500 lines) |
| `sdk/` | Full Agent SDK module (~50 files) |

**Startup Sequence:**
1. Parse CLI args (Commander.js)
2. Parallel prefetch: MDM policy, Keychain, API preconnect
3. Load config from `~/.claude/settings.json`
4. Initialize telemetry (GrowthBook, OpenTelemetry)
5. OAuth or API key auth
6. Create React/Ink app with AppStateProvider
7. Launch REPL screen

### 2.3 Query Engine: `src/QueryEngine.ts` (~46K lines)

The heart of the system. Handles:
- Streaming responses from Anthropic API
- Tool-call loops (detect tool_use blocks → execute → feed results back)
- Extended thinking mode with budget management
- Retry logic with exponential backoff
- Token counting & cost tracking
- Context management (conversation history)
- Permission system integration
- Message normalization for API

---

## 3. Tool System

### 3.1 Tool Interface: `src/Tool.ts` (~795 lines)

```typescript
type Tool<Input, Output, Progress> = {
  name: string
  aliases?: string[]
  call(args, context, canUseTool, parentMessage, onProgress): Promise<ToolResult<Output>>
  description(input, options): Promise<string>
  inputSchema: ZodSchema
  checkPermissions(input, context): Promise<PermissionResult>
  isConcurrencySafe(input): boolean
  isReadOnly(input): boolean
  isDestructive(input): boolean
  prompt(options): Promise<string>
  renderToolUseMessage(input, options): React.ReactNode
  renderToolResultMessage(content, options): React.ReactNode
  // + 10+ optional methods
}
```

### 3.2 Tool Registry: `src/tools.ts` (~400+ lines)

### 3.3 Tool Implementations (`src/tools/`)

Each tool in `src/tools/<ToolName>/`:
```
src/tools/FileEditTool/
├── FileEditTool.ts        # Main implementation
├── UI.tsx                 # Terminal rendering
├── prompt.ts              # System prompt injection
└── utils.ts               # Helpers
```

#### File System Tools
| Tool | Purpose |
|------|---------|
| FileReadTool | Read files (text, images, PDFs, notebooks) |
| FileWriteTool | Create/overwrite files |
| FileEditTool | Partial file modification via string replacement |
| GlobTool | Find files matching patterns |
| GrepTool | Content search with regex (ripgrep) |
| NotebookEditTool | Jupyter notebook cell editing |
| TodoWriteTool | Structured todo/task management |

#### Shell & Execution Tools
| Tool | Purpose |
|------|---------|
| BashTool | Execute shell commands |
| PowerShellTool | PowerShell (Windows) |
| REPLTool | Python/Node REPL sessions |

#### Agent & Orchestration Tools
| Tool | Purpose |
|------|---------|
| AgentTool | Spawn sub-agents |
| SendMessageTool | Inter-agent communication |
| TeamCreateTool / TeamDeleteTool | Agent teams |
| EnterPlanModeTool / ExitPlanModeTool | Plan-only mode |
| EnterWorktreeTool / ExitWorktreeTool | Git worktree isolation |
| SleepTool | Pause execution (proactive mode) |
| SyntheticOutputTool | Structured output |

#### Task Management Tools
| Tool | Purpose |
|------|---------|
| TaskCreateTool | Create background tasks |
| TaskUpdateTool | Update task status |
| TaskGetTool / TaskListTool | Query tasks |
| TaskOutputTool / TaskStopTool | Output & stop |

#### Web & Search Tools
| Tool | Purpose |
|------|---------|
| WebFetchTool | Fetch URL content |
| WebSearchTool | Web search |

#### MCP Tools
| Tool | Purpose |
|------|---------|
| MCPTool | Invoke tools on MCP servers |
| ListMcpResourcesTool / ReadMcpResourceTool | MCP resources |
| McpAuthTool | MCP authentication |
| ToolSearchTool | Discover deferred tools |

#### Integration & Utility Tools
| Tool | Purpose |
|------|---------|
| LSPTool | Language Server Protocol |
| SkillTool | Execute registered skills |
| AskUserQuestionTool | Prompt user |
| BriefTool | Generate summaries |
| ConfigTool | Read/modify settings |

---

## 4. Command System

### 4.1 Registry: `src/commands.ts` (~400+ lines)

Three command types:
1. **PromptCommand** — sends formatted prompt to LLM (e.g., /review, /commit, /plan)
2. **LocalCommand** — in-process, returns text (e.g., /cost, /version, /help)
3. **LocalJSXCommand** — in-process, returns React JSX (e.g., /doctor, /config)

### 4.2 Commands (~85 total, `src/commands/`)

```
src/commands/
├── add-dir/          ├── feedback/        ├── permissions/
├── agents/           ├── files/           ├── plan/
├── autofix-pr/       ├── help/            ├── plugin/
├── backfill-sessions/├── hooks/           ├── privacy-settings/
├── branch/           ├── ide/             ├── review.js
├── btw/              ├── init.js          ├── session/
├── clear/            ├── keybindings/     ├── share/
├── color/            ├── login/           ├── skills/
├── commit.js         ├── logout/          ├── status/
├── commit-push-pr.js ├── memory/          ├── summary/
├── compact/          ├── mcp/             ├── tasks/
├── config/           ├── mobile/          ├── teleport/
├── context/          ├── onboarding/      ├── theme/
├── copy/             ├── ... (30+ more)   ├── vim/
├── cost/             │                    └── ...
├── diff/             │
├── doctor/           │
├── effort/           │
├── exit/             │
├── export/           │
├── extra-usage/      │
├── fast/             │
```

---

## 5. React/Ink Terminal UI

### 5.1 Components (~140, `src/components/`)

```
src/components/
├── design-system/         # Layout & interactive primitives
├── Spinner.tsx            # Loading spinner
├── Message.tsx            # Message display
├── Editor.tsx             # Text editor
├── ToolUse.tsx            # Tool invocation display
├── ToolResult.tsx         # Tool result display
├── PermissionPrompt.tsx   # Permission dialog
├── NotificationBanner.tsx # Notifications
├── TabView.tsx            # Tab navigation
├── VirtualScroll.tsx      # Virtual scrolling for large outputs
└── ... (130+ more)
```

### 5.2 Screens (`src/screens/`)
- `REPL.tsx` — Main interactive conversation
- `Doctor.tsx` — Environment diagnostics
- `ResumeConversation.tsx` — Session restore UI

### 5.3 Hooks (~80, `src/hooks/`)

Categories:
- **Permission:** useCanUseTool, usePermissionContext, usePermissionModal
- **Session:** useSessionBackgrounding, useRemoteSession, useAssistantHistory
- **Input:** useTextInput, useVimInput, usePasteHandler, useInputBuffer
- **IDE:** useIDEIntegration, useIdeConnectionStatus, useDiffInIDE
- **Plugin/Skill:** useManagePlugins, useSkillsChange
- **Notifications:** useRateLimitNotification, useDeprecationWarning

---

## 6. State Management

### `src/state/AppStateStore.ts` (~500 lines)

```typescript
type AppState = {
  // Conversation
  messages: Message[]
  conversationId: UUID
  sessionId: string

  // Model & Config
  mainLoopModel: string
  thinkingConfig: ThinkingConfig

  // Permissions
  toolPermissionContext: ToolPermissionContext
  permissionMode: 'default' | 'plan' | 'bypassPermissions' | 'auto'

  // Tools & Commands
  tools: Tools
  commands: Command[]
  mcpServers: MCPServerConnection[]

  // UI
  currentScreen: 'repl' | 'doctor' | 'resume'
  isLoading: boolean
  activeToolUseIds: Set<string>

  // Tasks & Teams
  tasks: Map<string, TaskState>

  // Plugins & Skills
  loadedPlugins: Plugin[]
  registeredSkills: Skill[]

  // Memory
  memoryUsageBytes: number
  memoryLimit: number

  // Features
  isCoordinatorMode: boolean
  isProactiveMode: boolean
  isPlanMode: boolean
}
```

---

## 7. Key Data Structures

### Message Type Hierarchy (`src/types/message.ts`)

```typescript
type Message =
  | UserMessage { type: 'user', message, uuid, timestamp }
  | AssistantMessage { type: 'assistant', message: BetaMessage, uuid, requestId?, agentId? }
  | SystemMessage { type: 'system', subtype: 'informational'|'api_error'|'local_command'|..., uuid }
  | ProgressMessage<ToolProgressData> { type: 'progress', data, toolUseID }
  | AttachmentMessage { /* file attachments */ }
```

### ToolUseContext (`src/Tool.ts`)

```typescript
type ToolUseContext = {
  options: {
    commands, tools, mainLoopModel, mcpClients, mcpResources,
    agentDefinitions, thinkingConfig, maxBudgetUsd?,
    customSystemPrompt?, appendSystemPrompt?
  }
  abortController: AbortController
  readFileState: FileStateCache
  getAppState(): AppState
  setAppState(updater): void
  messages: Message[]
  agentId?: AgentId
  // + callbacks, progress tracking, file limits, etc.
}
```

### Permission Types (`src/types/permissions.ts`)

```typescript
type PermissionMode = 'default' | 'plan' | 'bypassPermissions' | 'auto'

type PermissionResult =
  | { behavior: 'allow', updatedInput? }
  | { behavior: 'deny', reason }
  | { behavior: 'prompt', prompt }

type ToolPermissionRule = { toolName, pattern, action: 'allow'|'deny'|'ask' }
```

---

## 8. Service Layer (`src/services/`)

| Directory | Purpose |
|-----------|---------|
| `api/` | Anthropic API client (claude.ts, client.ts, errors.ts, logging.ts) |
| `analytics/` | GrowthBook feature flags, OpenTelemetry tracing |
| `oauth/` | OAuth authentication |
| `mcp/` | MCP client & server integration |
| `plugins/` | Plugin loader & registry |
| `skills/` | Skill loader & registry |
| `extractMemories/` | Memory extraction from conversations |
| `teamMemorySync/` | Team knowledge sync |

---

## 9. Advanced Subsystems

### 9.1 Bridge / IDE Integration (`src/bridge/`, ~16 files)

Bidirectional channel to IDE extensions (VS Code, JetBrains):
- `bridgeMain.ts` — main bridge loop (~400 lines)
- `bridgeMessaging.ts` — message protocol (~300 lines)
- `bridgePermissionCallbacks.ts` — route permissions to IDE (~200 lines)
- `bridgeApi.ts` — API surface for IDE (~500 lines)
- `jwtUtils.ts` — JWT authentication
- **Feature Flag:** `BRIDGE_MODE`

### 9.2 Coordinator / Multi-Agent (`src/coordinator/`)
- `coordinatorMode.ts` — main coordinator loop
- `agentTeam.ts` — team management
- **Feature Flag:** `COORDINATOR_MODE`

### 9.3 Memory System (`src/memdir/`)
- Project memory: `./CLAUDE.md`
- User memory: `~/.claude/CLAUDE.md`
- Auto-extracted from conversations
- Team memory for shared knowledge

### 9.4 Plugin System (`src/services/plugins/`, `src/plugins/`)
- Discover, install, load, execute plugins
- Plugins contribute tools, commands, prompts
- Auto-update notifications

### 9.5 Skill System (`src/skills/`)
16 bundled skills: batch, claudeApi, debug, keybindings, loop, remember, simplify, stuck, verify, write-code, etc.

### 9.6 Task Management (`src/tasks/`)

| Task Type | Purpose |
|-----------|---------|
| LocalShellTask | Shell command background execution |
| LocalAgentTask | Sub-agent execution |
| RemoteAgentTask | Remote agent execution |
| InProcessTeammateTask | Parallel teammate agent |
| DreamTask | Background "dreaming" process |

### 9.7 Vim Mode (`src/vim/`)
- operators.ts, textObjects.ts, transitions.ts

### 9.8 Voice (`src/voice/`)
- Feature Flag: `VOICE_MODE`

---

## 10. Build System

### Bun Runtime (not Node.js)
- Native JSX/TSX support
- ES modules
- `bun:bundle` feature flags for dead-code elimination

### Build Process (`scripts/build-bundle.ts`)
- esbuild bundler
- Entrypoint: `src/entrypoints/cli.tsx`
- Output: single file `dist/claude.mjs`
- Target: node20, es2022, ESM

### Feature Flag Dead-Code Elimination

```typescript
import { feature } from 'bun:bundle'
if (feature('VOICE_MODE')) { /* stripped if VOICE_MODE=false */ }
```

**Available Flags:** PROACTIVE, KAIROS, BRIDGE_MODE, DAEMON, VOICE_MODE, AGENT_TRIGGERS, MONITOR_TOOL, COORDINATOR_MODE, WORKFLOW_SCRIPTS, CONTEXT_COLLAPSE, HISTORY_SNIP, OVERFLOW_TEST_TOOL, TERMINAL_PANEL, WEB_BROWSER_TOOL

### TypeScript Config
- target: ESNext, module: ESNext, moduleResolution: bundler
- jsx: react-jsx, strict: true
- React Compiler enabled

### Code Quality: Biome 1.9.0

---

## 11. External Dependencies

### Core AI/SDK
- `@anthropic-ai/sdk` ^0.39.0
- `@modelcontextprotocol/sdk` ^1.12.1

### Terminal UI
- `react` ^19.0.0, `react-reconciler` ^0.31.0
- `chalk` ^5.4.0, `ink` (via react-reconciler)
- `@xterm/xterm` ^5.5.0

### CLI & Build
- `commander` ^13.1.0
- `esbuild` ^0.25.0
- `typescript` ^5.7.0

### Validation & Data
- `zod` ^3.24.0
- `yaml` ^2.6.0
- `marked` ^15.0.0
- `highlight.js` ^11.11.0
- `lodash-es` ^4.17.21

### Shell & Process
- `node-pty` ^1.1.0
- `execa` ^9.5.0
- `tree-kill` ^1.2.2
- `chokidar` ^4.0.0

### Network
- `axios` ^1.7.0
- `ws` ^8.18.0
- `undici` ^7.3.0

### Analytics
- `@growthbook/growthbook` ^1.4.0
- `@opentelemetry/*` (~400KB)

### Other
- `semver` ^7.6.0, `diff` ^7.0.0, `fuse.js` ^7.0.0
- `qrcode` ^1.5.0, `ignore` ^6.0.0, `picomatch` ^4.0.0

---

## 12. Configuration & Environment

### Environment Variables (from .env.example)

**Auth:** ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL, CLAUDE_CODE_USE_BEDROCK/VERTEX/FOUNDRY

**Model:** ANTHROPIC_MODEL, ANTHROPIC_SMALL_FAST_MODEL, CLAUDE_CODE_SUBAGENT_MODEL

**Shell:** CLAUDE_CODE_SHELL, CLAUDE_CODE_SHELL_PREFIX, CLAUDE_CODE_TMPDIR

**Performance:** CLAUDE_CODE_MAX_OUTPUT_TOKENS, CLAUDE_CODE_IDLE_THRESHOLD_MINUTES

**Features:** CLAUDE_CODE_SIMPLE, CLAUDE_CODE_COORDINATOR_MODE, CLAUDE_CODE_PROACTIVE, CLAUDE_CODE_ENABLE_TASKS

**Settings Files:** `~/.claude/settings.json` (user), `.claude/settings.json` (project)

---

## 13. CLI Infrastructure (`src/cli/`)

| File | Purpose |
|------|---------|
| `exit.ts` | Exit handler |
| `print.ts` | Output formatting |
| `remoteIO.ts` | Remote I/O |
| `structuredIO.ts` | Structured output (JSON) |
| `ndjsonSafeStringify.ts` | NDJSON serialization |
| `update.ts` | CLI update checker |
| `handlers/auth.ts` | Auth handler |
| `handlers/agents.ts` | Agent handler |
| `handlers/autoMode.ts` | Auto mode handler |
| `transports/HybridTransport.ts` | WebSocket + SSE fallback |
| `transports/SSETransport.ts` | Server-sent events |
| `transports/WebSocketTransport.ts` | WebSocket client |
| `transports/ccrClient.ts` | CCR client communication |

---

## 14. MCP Server (`mcp-server/`)

Standalone MCP server for exploring Claude Code source:
- STDIO entrypoint (`src/index.ts`)
- HTTP server (`src/http.ts`)
- Vercel serverless (`api/vercelApp.ts`)
- Core server (`src/server.ts`)

---

## 15. Prompts Directory (`prompts/`)

16 sequential build-out prompts for reconstructing Claude Code:

| # | File | Topic |
|---|------|-------|
| 01 | install-bun-and-deps | Install Bun, dependencies |
| 02 | runtime-shims | bun:bundle shims |
| 03 | build-config | esbuild bundler setup |
| 04 | fix-mcp-server | Fix MCP server TS |
| 05 | env-and-auth | .env, auth stubs |
| 06 | ink-react-terminal-ui | Verify Ink/React pipeline |
| 07 | tool-system | Audit & wire 40+ tools |
| 08 | command-system | Audit & wire 50+ commands |
| 09 | query-engine | Get QueryEngine working |
| 10 | context-and-prompts | System prompt, context, memory |
| 11 | mcp-integration | MCP client/server |
| 12 | services-layer | Analytics, telemetry, limits |
| 13 | bridge-ide | IDE bridge layer |
| 14 | dev-runner | npm run dev |
| 15 | production-bundle | Production build |
| 16 | testing | Test infrastructure |

---

## 16. Architectural Patterns

### Tool Execution Flow
```
User Query → QueryEngine detects tool_use blocks
  → Tool registry lookup → Permission check
  → Tool.checkPermissions() → show dialog if needed
  → Tool.call(args, context) → execute
  → Result → serialize → include in next API request
  → Repeat until no more tool_use blocks
```

### Permission Flow
```
Tool invocation → validateInput() → checkPermissions()
  → mode == 'default' → show dialog
  → mode == 'plan' → show plan once
  → mode == 'bypassPermissions' → auto-allow
  → mode == 'auto' → ML classifier
```

### Feature Flag Dead-Code Elimination
```typescript
if (feature('BRIDGE_MODE')) {
  // Stripped at build time if BRIDGE_MODE=false
}
```

---

## 17. Key Files for Rust Rewrite

### Critical (Core Logic)
1. `src/QueryEngine.ts` (46K lines) — LLM loop, tool calling, streaming
2. `src/Tool.ts` — Tool interface definition
3. `src/commands.ts` — Command registry & dispatch
4. `src/tools.ts` — Tool registry
5. `src/state/AppStateStore.ts` — Global state
6. `src/types/message.ts` — Message types

### Critical (API & Services)
7. `src/services/api/claude.ts` — Anthropic API wrapper
8. `src/utils/auth.ts` — Authentication
9. `src/hooks/toolPermission/` — Permission system

### Important (Tool Implementations)
10. `src/tools/BashTool/` — Shell execution
11. `src/tools/FileEditTool/` — File editing
12. `src/tools/FileReadTool/` — File reading
13. `src/tools/AgentTool/` — Agent spawning

### Moderate Priority (Subsystems)
14. `src/bridge/` — IDE integration
15. `src/services/mcp/` — MCP client
16. `src/memdir/` — Memory system
17. `src/utils/permissions/` — Permission logic

---

## 18. Project Metrics

| Metric | Value |
|--------|-------|
| Total Files | ~2,172 |
| Source Lines (TypeScript) | 512,000+ |
| Tools | 40+ |
| Commands | 85+ |
| Components | ~140 |
| Hooks | ~80 |
| Services | 10+ |
| Key Dependencies | 30+ |
| Entry Points | 3 (CLI, MCP, SDK) |
| Feature Flags | 13+ |
| Build Output | Single binary + MCP server |

---

## 19. Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                   USER TERMINAL                     │
└───────────────────────┬─────────────────────────────┘
                        │
         ┌──────────────▼───────────────┐
         │  src/main.tsx (Commander.js) │
         │  Parse Args → Init React/Ink │
         └──────────────┬───────────────┘
                        │
         ┌──────────────▼───────────────┐
         │  src/replLauncher.tsx        │
         │  Init AppState → REPL Screen │
         └──────────────┬───────────────┘
                        │
    ┌───────────────────▼────────────────────┐
    │  src/screens/REPL.tsx (Main UI)       │
    │  ~140 Components + ~80 Hooks          │
    └───────────────────┬────────────────────┘
                        │
         ┌──────────────▼───────────────┐
         │  /command → commands.ts      │
         │  message → QueryEngine.ts    │
         └──────────────┬───────────────┘
                        │
         ┌──────────────▼───────────────────────┐
         │  QueryEngine.ts (~46K lines)         │
         │  Build prompt → Stream API → Tool    │
         │  loops → Retry → Token count → Cost  │
         └──────┬───────────┬───────────────────┘
                │           │
         ┌──────▼──┐  ┌────▼──────────┐
         │ API Call│  │ Tool Execution │
         │ claude  │  │ 40+ tools     │
         │ .ts     │  │ src/tools/    │
         └─────────┘  └───────────────┘
```
