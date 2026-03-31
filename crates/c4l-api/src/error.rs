//! API error types and classification.
//!
//! Maps from: leak-claude-code/src/services/api/errors.ts
//! Classifies errors as transient (retryable) or permanent.

/// Errors from the Anthropic API.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("overloaded: {message}")]
    Overloaded { message: String },

    #[error("authentication error: {message}")]
    Auth { message: String },

    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("server error ({status}): {message}")]
    Server { status: u16, message: String },

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("SSE parse error: {0}")]
    SseParse(String),

    #[error("stream error: {error_type}: {message}")]
    Stream { error_type: String, message: String },
}

impl ApiError {
    /// Whether this error is transient and the request should be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::Overloaded { .. } | Self::Server { status: 500.., .. }
        ) || matches!(self, Self::Network(e) if e.is_timeout() || e.is_connect())
    }

    /// Classify an HTTP status code + body into an ApiError.
    pub fn from_status(status: u16, body: &str) -> Self {
        // Try to parse the error body as JSON
        let message = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(String::from)
            })
            .unwrap_or_else(|| body.to_string());

        match status {
            401 => Self::Auth { message },
            400 => Self::InvalidRequest { message },
            429 => {
                // Parse retry-after from body or default to 30s
                let retry_after_ms = serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|v| {
                        v.get("error")
                            .and_then(|e| e.get("retry_after"))
                            .and_then(|r| r.as_f64())
                            .map(|s| (s * 1000.0) as u64)
                    })
                    .unwrap_or(30_000);
                Self::RateLimited { retry_after_ms }
            }
            529 => Self::Overloaded { message },
            _ if status >= 500 => Self::Server { status, message },
            _ => Self::InvalidRequest { message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_is_retryable() {
        let err = ApiError::RateLimited { retry_after_ms: 5000 };
        assert!(err.is_retryable());
    }

    #[test]
    fn auth_error_is_not_retryable() {
        let err = ApiError::Auth { message: "invalid key".into() };
        assert!(!err.is_retryable());
    }

    #[test]
    fn classify_429() {
        let body = r#"{"error":{"type":"rate_limit_error","message":"Too many requests"}}"#;
        let err = ApiError::from_status(429, body);
        assert!(matches!(err, ApiError::RateLimited { .. }));
        assert!(err.is_retryable());
    }

    #[test]
    fn classify_401() {
        let body = r#"{"error":{"type":"authentication_error","message":"Invalid API key"}}"#;
        let err = ApiError::from_status(401, body);
        assert!(matches!(err, ApiError::Auth { .. }));
        assert!(!err.is_retryable());
    }

    #[test]
    fn classify_500() {
        let err = ApiError::from_status(500, "Internal Server Error");
        assert!(matches!(err, ApiError::Server { status: 500, .. }));
        assert!(err.is_retryable());
    }
}
