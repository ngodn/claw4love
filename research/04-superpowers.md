# Superpowers: Development Workflow Enhancement Plugin

## Executive Summary

**Superpowers** (v5.0.6) is a sophisticated development workflow plugin for Claude Code CLI and other AI coding environments (Cursor, Codex, OpenCode, Gemini CLI). It implements a complete development methodology: Test-Driven Development, systematic debugging, collaborative planning, and autonomous multi-agent execution with quality gates.

**Core Purpose:** Transform ad-hoc AI coding into disciplined, reproducible workflows with hard gates that prevent rationalization and skipped steps.

---

## 1. Project Structure

```
superpowers/
├── skills/                          # 14 composable workflow skills
│   ├── using-superpowers/          # Meta: skill usage introduction
│   ├── brainstorming/              # Design ideation & validation
│   ├── writing-plans/              # Implementation planning
│   ├── writing-skills/             # Creating new skills (largest: 655 lines)
│   ├── test-driven-development/    # RED-GREEN-REFACTOR cycle (371 lines)
│   ├── systematic-debugging/       # Root cause investigation (296 lines)
│   ├── verification-before-completion/ # Evidence-based completion claims
│   ├── using-git-worktrees/        # Isolated workspace management
│   ├── subagent-driven-development/ # Task dispatch with two-stage review
│   ├── executing-plans/            # Inline plan execution
│   ├── dispatching-parallel-agents/ # Concurrent problem solving
│   ├── requesting-code-review/     # Review request workflow
│   ├── receiving-code-review/      # Technical review reception
│   └── finishing-a-development-branch/ # Merge/PR completion
├── hooks/                           # Plugin integration points
│   ├── hooks.json                  # Claude Code hook config
│   ├── hooks-cursor.json           # Cursor-specific hooks
│   ├── session-start               # SessionStart hook script (bash)
│   └── run-hook.cmd                # Windows hook runner
├── agents/
│   └── code-reviewer.md            # Code reviewer subagent template
├── commands/                       # Legacy command references
│   ├── brainstorm.md, execute-plan.md, write-plan.md
├── docs/                           # 72 markdown files
│   ├── README.codex.md, README.opencode.md
│   ├── plans/                      # Implementation plans
│   └── specs/                      # Design specifications
├── tests/                          # Multiple test frameworks
│   ├── brainstorm-server/          # Server unit & integration
│   ├── claude-code/                # Claude Code integration
│   ├── skill-triggering/           # Auto-invocation tests
│   ├── explicit-skill-requests/    # Manual invocation tests
│   ├── subagent-driven-dev/        # SDD execution tests
│   └── opencode/                   # OpenCode plugin tests
├── .opencode/plugins/superpowers.js # OpenCode plugin entry
├── gemini-extension.json            # Gemini CLI metadata
├── GEMINI.md                        # Gemini context injection
├── package.json                     # v5.0.6
├── README.md / CHANGELOG.md / RELEASE-NOTES.md
└── LICENSE                          # MIT
```

---

## 2. Skills Library (14 Skills, 3,157 Lines)

### Skill Metadata Format

Each skill at `skills/<name>/SKILL.md`:
```yaml
---
name: skill-name
description: "Use when [triggering conditions]"
---
```

### Skill Summary Table

| Skill | Lines | Category | Purpose |
|-------|-------|----------|---------|
| writing-skills | 655 | Meta | Skill authoring methodology |
| test-driven-development | 371 | Quality | RED-GREEN-REFACTOR cycle |
| systematic-debugging | 296 | Debugging | Root cause investigation |
| subagent-driven-development | 277 | Execution | Task dispatch + two-stage review |
| using-git-worktrees | 218 | Workflow | Isolated workspace setup |
| receiving-code-review | 213 | Quality | Technical review response |
| finishing-a-development-branch | 200 | Workflow | Merge/PR completion |
| dispatching-parallel-agents | 182 | Execution | Concurrent problem solving |
| brainstorming | 164 | Design | Socratic design refinement |
| writing-plans | 152 | Planning | Implementation planning |
| verification-before-completion | 139 | Quality | Evidence-based claims |
| using-superpowers | 115 | Meta | Skill system introduction |
| requesting-code-review | 105 | Quality | Review dispatch |
| executing-plans | 70 | Execution | Inline execution |

### Key Skills Detail

#### using-superpowers (115 lines)
- **Trigger:** Every conversation start (via SessionStart hook)
- **Content:** Priority hierarchy, The Rule ("invoke skills BEFORE any response"), Red Flags table (rationalizations), skill types (Rigid vs Flexible)

#### brainstorming (164 lines)
- **Hard Gate:** "Do NOT invoke any implementation skill until design is presented and approved"
- **Checklist:** Explore context → clarifying questions → propose 2-3 approaches → present design → write spec → self-review → user review → transition to implementation
- **Visual Companion:** Browser-based mockup tool via zero-dependency WebSocket server
- **Supporting files:** visual-companion.md, scripts/server.cjs, start-server.sh, stop-server.sh, frame-template.html, helper.js

#### test-driven-development (371 lines - largest behavioral skill)
- **Iron Law:** "NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST. Write code before test? Delete it."
- **Cycle:** RED (write failing test) → Verify RED → GREEN (simplest passing code) → Verify GREEN → REFACTOR
- **Red Flags:** Code before test, test passes immediately, "too simple to test", sunk cost fallacy
- **Verification Checklist:** Every function has test, watched each fail, minimal code, all pass, output pristine

#### systematic-debugging (296 lines)
- **Iron Law:** "NO FIXES WITHOUT ROOT CAUSE INVESTIGATION FIRST"
- **Four Phases:**
  1. Root Cause Investigation (read errors, reproduce, check changes, trace data flow)
  2. Pattern Analysis (find working examples, compare, identify differences)
  3. Hypothesis and Testing (single hypothesis, minimal test, verify)
  4. Implementation (failing test, single fix, verify, max 3 attempts then escalate)
- **Supporting:** root-cause-tracing.md, defense-in-depth.md, condition-based-waiting.md

#### writing-plans (152 lines)
- **Target persona:** "Engineer with zero context and questionable taste"
- **Plan structure:** Goal, Architecture, Tech Stack → Tasks with exact files/lines/code
- **"No Placeholders" Rule:** No TBD, TODO, "implement later", "similar to Task N"
- **Execution handoff:** Choice between subagent-driven (recommended) or inline

#### subagent-driven-development (277 lines)
- **Process per task:** Dispatch implementer → spec reviewer → code quality reviewer
- **Model selection:** Mechanical=cheap, Integration=standard, Architecture=capable
- **Status codes:** DONE, DONE_WITH_CONCERNS, NEEDS_CONTEXT, BLOCKED
- **Prompt templates:** implementer-prompt.md, spec-reviewer-prompt.md, code-quality-reviewer-prompt.md

#### verification-before-completion (139 lines)
- **Iron Law:** "NO COMPLETION CLAIMS WITHOUT FRESH VERIFICATION EVIDENCE"
- **Gate Function:** IDENTIFY command → RUN it → READ output → VERIFY claim → THEN claim
- **Red Flags:** "should", "probably", expressing satisfaction before running tests

#### writing-skills (655 lines - largest overall)
- **Principle:** "Writing skills IS TDD applied to process documentation"
- **TDD mapping:** Test=pressure scenario, Production=SKILL.md, RED=agent violates, GREEN=agent complies
- **Claude Search Optimization (CSO):** Description = WHEN to use, NOT what it does
- **Token efficiency:** Frequently-loaded skills <200 words

---

## 3. Hooks System

### SessionStart Hook

**hooks.json:**
```json
{
  "hooks": {
    "SessionStart": [{
      "matcher": "startup|clear|compact",
      "hooks": [{
        "type": "command",
        "command": "\"${CLAUDE_PLUGIN_ROOT}/hooks/run-hook.cmd\" session-start"
      }]
    }]
  }
}
```

**session-start script flow:**
1. Detect plugin root
2. Check for legacy skills directory warning
3. Read using-superpowers SKILL.md content
4. Escape for JSON (bash parameter substitution)
5. Wrap in EXTREMELY_IMPORTANT tags
6. Output as platform-appropriate JSON (Claude Code / Cursor / fallback)

**Result:** Every conversation gets full using-superpowers skill injected into system prompt.

---

## 4. Platform Integration

| Platform | Installation | Hooks | Skills | Subagents |
|----------|-------------|-------|--------|-----------|
| Claude Code | `/plugin install superpowers@claude-plugins-official` | hooks.json + bash | Skill tool | Yes |
| Cursor | `/add-plugin superpowers` | hooks-cursor.json | Plugin marketplace | Yes |
| OpenCode | opencode.json plugin array | .opencode/plugins/superpowers.js | Auto-registered | Yes |
| Codex | Symlink to ~/.agents/skills/ | Native discovery | Native discovery | Optional |
| Gemini CLI | `gemini extensions install` | GEMINI.md @imports | Auto-discovered | No (fallback to inline) |

---

## 5. Brainstorming Server

### Zero-Dependency WebSocket Server (v5.0.2+)

**File:** `skills/brainstorming/scripts/server.cjs`

- Custom WebSocket implementation (RFC 6455)
- Built-in Node.js modules only (http, fs, crypto)
- Random port in 49152-65535 range
- Auto-exit: owner process dies OR 30-min idle
- Session directory: content/ (HTML) + state/ (events.jsonl, server-info, pid)

---

## 6. Key Architectural Patterns

### Hard Gates (not gentle suggestions)
```
brainstorming: "Do NOT implement until design approved"
TDD: "NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"
debugging: "NO FIXES WITHOUT ROOT CAUSE INVESTIGATION"
verification: "NO COMPLETION CLAIMS WITHOUT FRESH EVIDENCE"
```

### Two-Stage Review Gates
1. **Spec Compliance:** Does code match specification?
2. **Code Quality:** Is implementation well-built?
(Sequential, not parallel — verify WHAT before evaluating HOW)

### Anti-Rationalization Tables
Every skill lists "Red Flags" — thoughts that indicate skipping discipline

### Subagent Context Isolation
- Subagents get only needed context (not session history)
- Prevents context window pollution
- Enables parallel execution

### Self-Review Checklists (v5.0.6)
Inline checklists replace subagent review loops: ~30 seconds vs ~25 minutes

---

## 7. Testing

| Suite | Framework | Location |
|-------|-----------|----------|
| Brainstorm server | JS/Node | tests/brainstorm-server/ |
| WebSocket protocol | JS/Node | tests/brainstorm-server/ws-protocol.test.js |
| Windows lifecycle | Bash | tests/brainstorm-server/windows-lifecycle.test.sh |
| Skill triggering | Bash | tests/skill-triggering/ |
| Explicit requests | Bash | tests/explicit-skill-requests/ |
| SDD execution | Mixed | tests/subagent-driven-dev/ |
| OpenCode plugin | Bash | tests/opencode/ |
| Claude Code | Python | tests/claude-code/ |

---

## 8. Version History

| Version | Date | Key Changes |
|---------|------|-------------|
| 5.0.6 | 2026-03-24 | Self-review checklists, Codex compat, PID fixes |
| 5.0.5 | 2026-03-17 | ESM fix (server.cjs), Windows PID skip, stop reliability |
| 5.0.4 | 2026-03-16 | Single-pass review, iteration limits, OpenCode install |
| 5.0.3 | 2026-03-15 | Cursor support, bash 5.3+ fix, POSIX safety |
| 5.0.2 | 2026-03-11 | Zero-dep server, removed vendored node_modules |
| 5.0.1 | 2026-03-10 | Gemini CLI support, server relocation |
