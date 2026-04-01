# Crush: Authentication and API Architecture Analysis

## Overview

Crush is a Go-based AI coding assistant by Charmbracelet. It uses a provider-agnostic architecture with multiple auth methods. Key finding: **crush explicitly dropped Claude Code subscription OAuth support** — they only support direct API keys for Anthropic now.

## Authentication Methods

### 1. Direct API Key
- Stored in config: `"api_key": "sk-ant-..."` or `"api_key": "$ANTHROPIC_API_KEY"`
- Environment variable templates resolved at runtime
- Re-resolved on 401 errors

### 2. OAuth2 Tokens
- Used for GitHub Copilot and Hyper (Charmbracelet's proxy)
- Token structure: access_token, refresh_token, expires_in, expires_at
- Auto-refresh on expiry

### 3. Hyper Proxy
- Charmbracelet's API proxy: `https://hyper.charm.land/api/v1/fantasy`
- Device auth flow for token acquisition
- Acts as bridge to multiple providers

## Anthropic-Specific Auth

File: `internal/agent/coordinator.go` lines 598-628

```go
func buildAnthropicProvider(baseURL, apiKey string, headers map[string]string) {
    switch {
    case strings.HasPrefix(apiKey, "Bearer "):
        headers["Authorization"] = apiKey  // OAuth-style
    case apiKey != "":
        opts = append(opts, anthropic.WithAPIKey(apiKey))  // x-api-key header
    }
}
```

Two modes:
- API key starting with "Bearer " → sent as Authorization header
- Regular API key → sent as x-api-key header via Anthropic SDK

No special betas, no metadata.user_id, no thinking config — they use the standard Anthropic API with plain API keys, not the Claude Code subscription endpoint.

## Claude Code OAuth: Explicitly Removed

File: `internal/config/load.go` lines 238-240

```go
case p.ID == catwalk.InferenceProviderAnthropic && config.OAuthToken != nil:
    // Claude Code subscription is not supported anymore. Remove to show onboarding.
    store.RemoveConfigField(ScopeGlobal, "providers.anthropic")
```

Crush used to support Claude Code subscription OAuth but removed it. The comment says "not supported anymore" — likely because Anthropic's OAuth endpoint requires specific headers (claude-code-20250219 beta, metadata.user_id, thinking config, Stainless SDK headers) that are tightly coupled to the official Claude Code CLI.

## Config Storage

- Global: `~/.local/share/crush/crush.json`
- Workspace: `.crush/crush.json`
- Permissions: 0o600

## Key Takeaway for claw4love

Crush's approach doesn't help us with OAuth subscription auth. They gave up on it and use plain API keys. Our session bootstrap approach (spawning real Claude Code CLI through a transparent proxy) is the right strategy since the OAuth endpoint requires very specific headers and metadata that only the official CLI sends correctly.
