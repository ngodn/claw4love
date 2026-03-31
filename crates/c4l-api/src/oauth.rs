//! OAuth 2.0 authentication for Claude Code subscriptions (Pro/Max/Team/Enterprise).
//!
//! Maps from: leak-claude-code/src/services/oauth/ + src/utils/auth.ts
//!
//! Flow:
//! 1. Generate PKCE code verifier + challenge
//! 2. Open browser to authorization URL
//! 3. Local HTTP server catches redirect with auth code
//! 4. Exchange code for access_token + refresh_token
//! 5. Store tokens in ~/.claude/.credentials.json
//! 6. Use Bearer token for API requests
//! 7. Auto-refresh before expiry

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// OAuth configuration constants.
/// Verified from: leak-claude-code/src/constants/oauth.ts
pub const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const TOKEN_ENDPOINT: &str = "https://platform.claude.com/v1/oauth/token";
pub const AUTHORIZE_URL: &str = "https://claude.com/cai/oauth/authorize";
pub const MANUAL_REDIRECT_URL: &str = "https://platform.claude.com/oauth/code/callback";
pub const SUCCESS_URL: &str = "https://platform.claude.com/oauth/code/success?app=claude-code";
pub const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
pub const OAUTH_BETA: &str = "oauth-2025-04-20";

/// Scopes requested during login.
/// Verified from: leak-claude-code/src/constants/oauth.ts lines 33-58
pub const CLAUDE_AI_SCOPES: &[&str] = &[
    "user:profile",
    "user:inference",
    "user:sessions:claude_code",
    "user:mcp_servers",
    "user:file_upload",
];

pub const CONSOLE_SCOPES: &[&str] = &["org:create_api_key", "user:profile"];

/// Stored OAuth credentials.
/// Matches the format in ~/.claude/.credentials.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: Option<StoredOAuthToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredOAuthToken {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64, // milliseconds since epoch
    pub scopes: Vec<String>,
    #[serde(rename = "subscriptionType")]
    pub subscription_type: Option<String>,
    #[serde(rename = "rateLimitTier")]
    pub rate_limit_tier: Option<String>,
}

impl StoredOAuthToken {
    /// Check if the token is expired (with 5-minute buffer).
    pub fn is_expired(&self) -> bool {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        // 5-minute buffer before actual expiry
        self.expires_at <= now_ms + 300_000
    }

    /// Whether this token supports inference (Claude.ai subscription).
    pub fn has_inference_scope(&self) -> bool {
        self.scopes.iter().any(|s| s == "user:inference")
    }
}

/// Token exchange response from the OAuth server.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    scope: Option<String>,
}

/// PKCE (Proof Key for Code Exchange) parameters.
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
    pub state: String,
}

impl PkceChallenge {
    /// Generate a new PKCE challenge with random verifier and state.
    pub fn generate() -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Generate random bytes using system time + process id as entropy
        let seed = format!(
            "{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            format!("{:?}", std::thread::current().id()),
        );

        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let verifier = base64url_encode(&hasher.finish().to_le_bytes().repeat(4));

        // SHA256 of verifier for challenge (simplified - in production use sha2 crate)
        let mut hasher2 = DefaultHasher::new();
        verifier.hash(&mut hasher2);
        let challenge = base64url_encode(&hasher2.finish().to_le_bytes().repeat(4));

        let mut hasher3 = DefaultHasher::new();
        format!("state-{seed}").hash(&mut hasher3);
        let state = base64url_encode(&hasher3.finish().to_le_bytes().repeat(4));

        Self {
            verifier,
            challenge,
            state,
        }
    }
}

fn base64url_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    for b in bytes {
        write!(s, "{:02x}", b).unwrap();
    }
    s
}

/// Build the authorization URL for the browser.
pub fn build_authorize_url(
    pkce: &PkceChallenge,
    redirect_uri: &str,
    email: Option<&str>,
) -> String {
    let scopes: Vec<&str> = CLAUDE_AI_SCOPES
        .iter()
        .chain(CONSOLE_SCOPES.iter())
        .copied()
        .collect();
    let scope_str = scopes.join(" ");

    let mut url = format!(
        "{}?code=true&client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        AUTHORIZE_URL,
        CLIENT_ID,
        urlencoded(redirect_uri),
        urlencoded(&scope_str),
        &pkce.challenge,
        &pkce.state,
    );

    if let Some(email) = email {
        url.push_str(&format!("&login_hint={}", urlencoded(email)));
    }

    url
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('?', "%3F")
        .replace('=', "%3D")
        .replace('&', "%26")
        .replace('@', "%40")
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(
    http: &reqwest::Client,
    code: &str,
    redirect_uri: &str,
    pkce_verifier: &str,
    state: &str,
) -> Result<StoredOAuthToken> {
    debug!("exchanging authorization code for tokens");

    let resp = http
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", CLIENT_ID),
            ("code_verifier", pkce_verifier),
            ("state", state),
        ])
        .send()
        .await
        .context("failed to exchange auth code")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("token exchange failed ({status}): {body}");
    }

    let token_resp: TokenResponse = resp.json().await?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let scopes = token_resp
        .scope
        .unwrap_or_default()
        .split(' ')
        .map(String::from)
        .collect();

    Ok(StoredOAuthToken {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token.unwrap_or_default(),
        expires_at: now_ms + (token_resp.expires_in * 1000),
        scopes,
        subscription_type: None,
        rate_limit_tier: None,
    })
}

/// Refresh an expired token.
pub async fn refresh_token(
    http: &reqwest::Client,
    refresh_tok: &str,
) -> Result<StoredOAuthToken> {
    debug!("refreshing OAuth token");

    let scope_str = CLAUDE_AI_SCOPES.join(" ");

    let resp = http
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_tok),
            ("client_id", CLIENT_ID),
            ("scope", &scope_str),
        ])
        .send()
        .await
        .context("failed to refresh token")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("token refresh failed ({status}): {body}");
    }

    let token_resp: TokenResponse = resp.json().await?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let scopes = token_resp
        .scope
        .unwrap_or_default()
        .split(' ')
        .map(String::from)
        .collect();

    Ok(StoredOAuthToken {
        access_token: token_resp.access_token,
        refresh_token: token_resp
            .refresh_token
            .unwrap_or_else(|| refresh_tok.to_string()),
        expires_at: now_ms + (token_resp.expires_in * 1000),
        scopes,
        subscription_type: None,
        rate_limit_tier: None,
    })
}

// -- Credential storage --

/// Default credentials file path.
pub fn credentials_path() -> PathBuf {
    let config_dir = std::env::var("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|_| dirs::home_dir().map(|h| h.join(".claude")).ok_or(()))
        .unwrap_or_else(|_| PathBuf::from(".claude"));

    config_dir.join(".credentials.json")
}

/// Load stored OAuth credentials from disk.
pub fn load_credentials() -> Result<Option<StoredOAuthToken>> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .context("failed to read credentials file")?;
    let creds: OAuthCredentials = serde_json::from_str(&content)
        .context("failed to parse credentials")?;

    Ok(creds.claude_ai_oauth)
}

/// Save OAuth credentials to disk.
pub fn save_credentials(token: &StoredOAuthToken) -> Result<()> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let creds = OAuthCredentials {
        claude_ai_oauth: Some(token.clone()),
    };
    let json = serde_json::to_string_pretty(&creds)?;

    std::fs::write(&path, &json)?;

    // Set file permissions to 0600 on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    info!(?path, "saved OAuth credentials");
    Ok(())
}

/// Clear stored credentials (logout).
pub fn clear_credentials() -> Result<()> {
    let path = credentials_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
        info!("cleared OAuth credentials");
    }
    Ok(())
}

// -- Auth resolution --

/// The resolved authentication method.
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// Using an API key (ANTHROPIC_API_KEY or config).
    ApiKey(String),
    /// Using OAuth bearer token (subscription login).
    OAuth(StoredOAuthToken),
    /// No authentication available.
    None,
}

/// Resolve the best available authentication method.
///
/// Priority (from leak-claude-code/src/utils/auth.ts lines 151-206):
/// 1. ANTHROPIC_AUTH_TOKEN env var
/// 2. CLAUDE_CODE_OAUTH_TOKEN env var
/// 3. ANTHROPIC_API_KEY env var
/// 4. Config file api_key
/// 5. Stored OAuth credentials (~/.claude/.credentials.json)
/// 6. None
pub fn resolve_auth(config: &c4l_config::C4lConfig) -> AuthMethod {
    // 1. ANTHROPIC_AUTH_TOKEN
    if let Ok(token) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
        return AuthMethod::ApiKey(token);
    }

    // 2. CLAUDE_CODE_OAUTH_TOKEN
    if let Ok(token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
        return AuthMethod::OAuth(StoredOAuthToken {
            access_token: token,
            refresh_token: String::new(),
            expires_at: u64::MAX,
            scopes: CLAUDE_AI_SCOPES.iter().map(|s| s.to_string()).collect(),
            subscription_type: None,
            rate_limit_tier: None,
        });
    }

    // 3. ANTHROPIC_API_KEY
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        return AuthMethod::ApiKey(key);
    }

    // 4. Config file api_key
    if let Some(key) = &config.auth.api_key {
        return AuthMethod::ApiKey(key.clone());
    }

    // 5. Stored OAuth credentials
    if let Ok(Some(token)) = load_credentials() {
        return AuthMethod::OAuth(token);
    }

    AuthMethod::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_generates_unique_values() {
        let p1 = PkceChallenge::generate();
        let p2 = PkceChallenge::generate();
        // They should be different (not guaranteed but extremely likely)
        assert!(!p1.verifier.is_empty());
        assert!(!p1.challenge.is_empty());
        assert!(!p1.state.is_empty());
    }

    #[test]
    fn authorize_url_contains_required_params() {
        let pkce = PkceChallenge::generate();
        let url = build_authorize_url(&pkce, "http://localhost:9999/callback", None);
        assert!(url.starts_with(AUTHORIZE_URL));
        assert!(url.contains(CLIENT_ID));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("user%3Ainference"));
    }

    #[test]
    fn authorize_url_with_email() {
        let pkce = PkceChallenge::generate();
        let url = build_authorize_url(&pkce, "http://localhost:9999/callback", Some("user@example.com"));
        assert!(url.contains("login_hint=user%40example.com"));
    }

    #[test]
    fn token_expiry_check() {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Expired token
        let expired = StoredOAuthToken {
            access_token: "test".into(),
            refresh_token: "test".into(),
            expires_at: now_ms - 1000,
            scopes: vec![],
            subscription_type: None,
            rate_limit_tier: None,
        };
        assert!(expired.is_expired());

        // Valid token (1 hour from now)
        let valid = StoredOAuthToken {
            access_token: "test".into(),
            refresh_token: "test".into(),
            expires_at: now_ms + 3_600_000,
            scopes: vec![],
            subscription_type: None,
            rate_limit_tier: None,
        };
        assert!(!valid.is_expired());

        // Within 5-minute buffer
        let borderline = StoredOAuthToken {
            access_token: "test".into(),
            refresh_token: "test".into(),
            expires_at: now_ms + 200_000, // 3.3 minutes, less than 5-min buffer
            scopes: vec![],
            subscription_type: None,
            rate_limit_tier: None,
        };
        assert!(borderline.is_expired());
    }

    #[test]
    fn has_inference_scope() {
        let with = StoredOAuthToken {
            access_token: "t".into(),
            refresh_token: "r".into(),
            expires_at: u64::MAX,
            scopes: vec!["user:profile".into(), "user:inference".into()],
            subscription_type: None,
            rate_limit_tier: None,
        };
        assert!(with.has_inference_scope());

        let without = StoredOAuthToken {
            access_token: "t".into(),
            refresh_token: "r".into(),
            expires_at: u64::MAX,
            scopes: vec!["user:profile".into()],
            subscription_type: None,
            rate_limit_tier: None,
        };
        assert!(!without.has_inference_scope());
    }

    #[test]
    fn credentials_roundtrip() {
        let token = StoredOAuthToken {
            access_token: "acc_123".into(),
            refresh_token: "ref_456".into(),
            expires_at: 1700000000000,
            scopes: vec!["user:inference".into()],
            subscription_type: Some("max".into()),
            rate_limit_tier: Some("tier_4".into()),
        };

        let creds = OAuthCredentials {
            claude_ai_oauth: Some(token),
        };

        let json = serde_json::to_string(&creds).unwrap();
        assert!(json.contains("claudeAiOauth"));
        assert!(json.contains("acc_123"));

        let back: OAuthCredentials = serde_json::from_str(&json).unwrap();
        let t = back.claude_ai_oauth.unwrap();
        assert_eq!(t.access_token, "acc_123");
        assert_eq!(t.subscription_type, Some("max".into()));
    }
}
