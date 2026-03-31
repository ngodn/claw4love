# Claude Code CLI: OAuth Authentication Flow

Research findings from leak-claude-code source analysis.

## Overview

Claude Code CLI supports two auth methods:
1. API key (ANTHROPIC_API_KEY) for direct API access
2. OAuth login for Claude Pro/Max/Team/Enterprise subscriptions

The OAuth flow uses PKCE (S256) with a browser-based authorization and local callback server.

## OAuth Configuration

Source: `leak-claude-code/src/constants/oauth.ts`

| Field | Value |
|-------|-------|
| Client ID | `9d1c250a-e61b-44d9-88ed-5944d1962f5e` |
| Token endpoint | `https://platform.claude.com/v1/oauth/token` |
| Authorize URL (Claude.ai) | `https://claude.com/cai/oauth/authorize` |
| Authorize URL (Console) | `https://platform.claude.com/oauth/authorize` |
| API base URL | `https://api.anthropic.com` |
| Profile endpoint | `https://api.anthropic.com/api/oauth/profile` |
| API key creation | `https://api.anthropic.com/api/oauth/claude_cli/create_api_key` |
| Roles endpoint | `https://api.anthropic.com/api/oauth/claude_cli/roles` |
| Manual redirect URL | `https://platform.claude.com/oauth/code/callback` |
| Success URL | `https://platform.claude.com/oauth/code/success?app=claude-code` |
| Beta header | `anthropic-beta: oauth-2025-04-20` |

## Scopes

Claude.ai scopes: `user:profile`, `user:inference`, `user:sessions:claude_code`, `user:mcp_servers`, `user:file_upload`

Console scopes: `org:create_api_key`, `user:profile`

All scopes are requested at login to handle both paths.

## PKCE Flow

Source: `leak-claude-code/src/services/oauth/crypto.ts`

- Code verifier: 32 random bytes, base64url-encoded
- Code challenge: SHA256 of verifier, base64url-encoded
- State parameter: 32 random bytes, base64url-encoded
- Challenge method: S256

## Authorization URL Parameters

Source: `leak-claude-code/src/services/oauth/client.ts` lines 46-105

```
?code=true
&client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e
&response_type=code
&redirect_uri=http://localhost:{port}/callback  (or manual redirect URL)
&scope={all scopes joined by space}
&code_challenge={challenge}
&code_challenge_method=S256
&state={state}
&login_hint={email}           (optional)
&login_method={sso|magic_link|google}  (optional)
&orgUUID={uuid}               (optional)
```

## Two Auth Code Delivery Methods

Source: `leak-claude-code/src/services/oauth/index.ts`

1. Automatic: browser redirects to `http://localhost:{port}/callback`, local HTTP server captures code
2. Manual: user copies authorization code from browser and pastes into CLI

## Token Exchange

Source: `leak-claude-code/src/services/oauth/client.ts` lines 107-144

```
POST https://platform.claude.com/v1/oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code
&code={authorization_code}
&redirect_uri={redirect_uri}
&client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e
&code_verifier={verifier}
&state={state}
```

Response:
- `access_token`: Bearer token for API requests
- `refresh_token`: for token refresh without re-login
- `expires_in`: seconds until expiry
- `scope`: space-separated scopes granted
- `account`: `{uuid, email_address}`
- `organization`: `{uuid}`

## Token Refresh

Source: `leak-claude-code/src/services/oauth/client.ts` lines 146-274

```
POST https://platform.claude.com/v1/oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token
&refresh_token={refresh_token}
&client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e
&scope=user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload
```

Token expiry buffer: 5 minutes before actual expiry (clock skew safety).

## Authenticated API Requests

Source: `leak-claude-code/src/services/api/client.ts`

For OAuth users:
```
Authorization: Bearer {access_token}
anthropic-beta: oauth-2025-04-20
Content-Type: application/json
```

No `x-api-key` header is sent for OAuth users.

For API key users:
```
x-api-key: {api_key}
Content-Type: application/json
```

## Token Storage

Source: `leak-claude-code/src/utils/auth.ts` lines 1194-1253, `src/utils/secureStorage/`

Storage format (`~/.claude/.credentials.json`):
```json
{
  "claudeAiOauth": {
    "accessToken": "...",
    "refreshToken": "...",
    "expiresAt": 1700000000000,
    "scopes": ["user:profile", "user:inference", ...],
    "subscriptionType": "max",
    "rateLimitTier": "tier_4"
  }
}
```

Platform-specific storage:
- macOS: Primary Keychain (service: `Claude-Code`), fallback `~/.claude/.credentials.json`
- Linux: `~/.claude/.credentials.json` (mode 0600)
- Windows: `~/.claude/.credentials.json` (mode 0600)

Config directory: `CLAUDE_CONFIG_DIR` env var or `~/.claude`

## Auth Resolution Priority

Source: `leak-claude-code/src/utils/auth.ts` lines 151-206

1. `ANTHROPIC_AUTH_TOKEN` env var
2. `CLAUDE_CODE_OAUTH_TOKEN` env var (current session from CCR)
3. `CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR` env var (CCR file descriptor)
4. CCR well-known path `/home/claude/.claude/remote/.oauth_token`
5. `apiKeyHelper` from settings (executed command returning token)
6. `ANTHROPIC_API_KEY` env var
7. Config file api_key
8. Stored OAuth credentials from `~/.claude/.credentials.json`
9. None

## Console Users (API Key Creation)

Source: `leak-claude-code/src/services/oauth/client.ts` lines 311-342

Console users without `user:inference` scope cannot use OAuth tokens directly. Instead:

```
POST https://api.anthropic.com/api/oauth/claude_cli/create_api_key
Authorization: Bearer {access_token}
```

Response: `{raw_key: "sk-ant-..."}`

The generated API key is stored for subsequent API calls.

## Relevant Environment Variables

OAuth:
- `CLAUDE_CODE_OAUTH_TOKEN`: direct OAuth token (no refresh)
- `CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR`: FD pointing to token (CCR)
- `CLAUDE_CODE_OAUTH_REFRESH_TOKEN`: refresh token for fast login
- `CLAUDE_CODE_OAUTH_SCOPES`: scopes for refresh token
- `CLAUDE_CODE_OAUTH_CLIENT_ID`: override client ID

Config:
- `CLAUDE_CONFIG_DIR`: override config directory

Overrides:
- `CLAUDE_CODE_CUSTOM_OAUTH_URL`: FedStart deployment URL
- `USE_STAGING_OAUTH`: staging endpoints (ant builds)
- `USE_LOCAL_OAUTH`: localhost endpoints (ant builds)

## Error Handling

Source: `leak-claude-code/src/utils/http.ts`

- `withOAuth401Retry`: auto-refresh token on 401, retry failed request
- Token expiry buffer: 5 minutes before actual expiry for clock skew
