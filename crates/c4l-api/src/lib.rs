//! Anthropic Messages API client with streaming support.
//!
//! Maps from: leak-claude-code/src/services/api/claude.ts (3,419 lines)
//! We implement the core: HTTP client, SSE streaming, retry, error classification.

pub mod types;
pub mod error;
pub mod client;
pub mod sse;
pub mod oauth;

pub use client::AnthropicClient;
pub use error::ApiError;
pub use oauth::{AuthMethod, StoredOAuthToken};
pub use types::*;
