# claw4love

An attempt to rewrite leaked Claude Code CLI in Rust. **This project is discontinued.**

## What Happened

We built a complete Rust workspace with 13 crates and 165 tests covering the full Claude Code architecture: API client, query engine with tool-call loop, 6 tools, TUI, slash commands, plugin/skill/hook system, MCP client, session management, and token optimization pipeline.

Then we hit a wall. Anthropic's subscription API (used by Claude Pro/Max/Team/Enterprise) is locked to the official client. It requires specific undocumented headers (`claude-code-20250219` beta, `metadata.user_id` with device/account/session IDs, Stainless SDK fingerprint headers) that are tightly coupled to the official Claude Code CLI. Direct API calls from any other client get rejected with vague 429 errors, even with valid tokens and correct request format.

We confirmed this by:
- Intercepting the exact request the official CLI sends via a transparent proxy
- Replaying the identical request with curl, rejected
- Finding that Crush/opencode (Charmbracelet's Go-based coding assistant) explicitly dropped Claude Code subscription OAuth support with the comment "Claude Code subscription is not supported anymore"

The API appears to validate requests at the transport level (TLS fingerprinting, HTTP/2 behavior, or similar) beyond just headers and body content.

## Why This Matters

The Rust codebase itself works. The architecture is sound, the types match the original TypeScript, the tool system executes properly, the TUI renders. If you have a pay-per-token Anthropic API key (`ANTHROPIC_API_KEY`), this would work as a standalone CLI. The problem is specifically with subscription-based authentication (Pro/Max/Team/Enterprise plans), which is what most Claude Code users actually have.

This is not a problem unique to this project. Every third-party project that has tried to use Claude subscription tokens directly has either been blocked or given up:

- **Crush/opencode** (Charmbracelet) removed Claude Code OAuth support entirely, now only accepts direct API keys for Anthropic
- **CLIProxyAPI** solves the problem by wrapping the official `claude --print` CLI as a backend, meaning it still depends entirely on the official Claude Code binary being installed and logged in
- **claude-code-proxy** and its many forks all take the same wrapper approach, piping requests through the real CLI
- **Anthropic's Agent SDK** explicitly requires API key authentication and does not support OAuth tokens from subscription accounts

In other words, there is no known way to use a Claude subscription token to make API calls without going through the official Claude Code CLI. The subscription is tied to the client, not just the token.

## The Right Approach

The projects that actually succeed at improving Claude Code don't replace it. They extend it.

**RTK (Rust Token Killer)** sits as a proxy layer between Claude Code and the shell, filters tool output before it reaches the LLM context window. Saves 60-90% tokens without touching the API. It works because it operates at the tool output level, not the API level.

**Superpowers** injects skills and hooks into Claude Code's existing plugin system. It changes how Claude behaves (TDD enforcement, systematic debugging, verification gates) without replacing the client. It works because Claude Code has a well-designed extension architecture.

**Everything Claude Code (ECC)** provides agents, commands, rules, and workflow automation through Claude Code's native plugin/skill/hook/command system. 30 agents, 136 skills, 60 commands. It works because it plugs into existing extension points.

**CLIProxyAPI** wraps `claude --print --output-format stream-json` as an OpenAI-compatible API endpoint so other tools (Cursor, Cline, Factory) can use your Max subscription. It works, but it still requires the official Claude Code CLI installed and authenticated underneath.

The lesson is simple: Claude Code's value is not in the CLI binary. It's in the API access, the model, and the tool-call loop. The CLI is just the delivery mechanism, and Anthropic controls it. You can build better workflows, better token efficiency, better UI on top of it, but you cannot replace the client itself if you want to use a subscription plan.

## What's Here

The codebase is still a useful reference for anyone studying Claude Code's architecture or building Rust-based AI tooling.

### Research (research/)

Component-by-component analysis of the Claude Code ecosystem:
- Original TypeScript source (512K LOC)
- Python and Rust porting attempts
- RTK token optimization proxy (19K LOC Rust)
- ECC and Superpowers plugin ecosystems
- OAuth authentication flow (endpoints, scopes, token storage)
- Crush authentication architecture and why they dropped OAuth

### Porting Plans (porting/)

Phase-by-phase implementation plans with exact Rust type definitions mapped from TypeScript source. Useful as architecture documentation even if the project is not continuing.

### Rust Crates (crates/)

13 crates, 165 tests:

| Crate | What It Does |
|-------|-------------|
| c4l-types | Messages, permissions, tools, commands, sessions |
| c4l-config | Layered config loading (TOML + env vars + defaults) |
| c4l-cli | CLI with login, doctor, sessions, REPL, oneshot |
| c4l-api | Anthropic API client with SSE streaming, retry, OAuth, session bootstrap |
| c4l-engine | Query engine with tool-call loop and event streaming |
| c4l-tools | Tool trait, registry, Bash/Read/Edit/Write/Glob/Grep |
| c4l-state | SQLite session store, cost tracking, shared app state |
| c4l-commands | Slash command trait, registry, 11 built-in commands |
| c4l-tui | Ratatui REPL with streaming, input, permissions |
| c4l-mcp | MCP client with STDIO transport and JSON-RPC |
| c4l-plugins | Skill parsing, hook execution, memory loading, plugin discovery |
| c4l-bridge | IDE bridge protocol types |
| c4l-utils | Token filter pipeline, ANSI stripping, git worktrees |

## Building (if you want to explore)

```
cargo build --workspace
cargo test --workspace
cargo run -p c4l-cli -- doctor
```

## License

MIT
