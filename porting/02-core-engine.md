# Phase 1: Core Engine — API Client, QueryEngine, Streaming

## What This Phase Delivers

A working LLM conversation loop: send messages to Claude API, stream responses, detect tool_use blocks, execute tools, feed results back. This is the heart of Claude Code — `QueryEngine.ts` (~46K lines in TypeScript).

## Crates

### c4l-api (Anthropic API Client)

Maps from: `src/services/api/claude.ts` (3,419 lines)

```rust
// crates/c4l-api/src/lib.rs

/// Anthropic Messages API client with streaming support
pub struct AnthropicClient {
    http: reqwest::Client,
    config: ApiConfig,
}

pub struct ApiConfig {
    pub api_key: String,
    pub base_url: String,       // default: "https://api.anthropic.com"
    pub model: String,
    pub max_tokens: u32,
    pub api_version: String,    // "2023-06-01"
    pub betas: Vec<String>,     // ["extended-thinking", ...]
}

/// Streaming event types (Server-Sent Events from Anthropic API)
/// Maps from: Anthropic SDK stream events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageResponse },
    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: usize, content_block: ContentBlock },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaData, usage: Option<UsageData> },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: ApiError },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

impl AnthropicClient {
    pub fn new(config: ApiConfig) -> Self;

    /// Stream a messages request, yielding events
    /// Maps from: claude.ts streaming logic
    pub async fn stream_messages(
        &self,
        messages: &[ApiMessage],
        system: &str,
        tools: &[ApiToolDef],
    ) -> Result<impl Stream<Item = Result<StreamEvent>>>;

    /// Non-streaming request (for simple cases)
    pub async fn create_message(
        &self,
        messages: &[ApiMessage],
        system: &str,
        tools: &[ApiToolDef],
    ) -> Result<MessageResponse>;
}

/// Retry logic with exponential backoff
/// Maps from: src/services/api/errors.ts
pub struct RetryPolicy {
    pub max_retries: u32,       // default: 3
    pub initial_delay_ms: u64,  // default: 1000
    pub max_delay_ms: u64,      // default: 30000
    pub backoff_factor: f64,    // default: 2.0
}

/// API error classification
/// Maps from: src/services/api/errors.ts
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("overloaded: {message}")]
    Overloaded { message: String },
    #[error("auth error: {message}")]
    AuthError { message: String },
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },
    #[error("server error: {status} {message}")]
    ServerError { status: u16, message: String },
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}
```

### c4l-engine (QueryEngine)

Maps from: `src/QueryEngine.ts` (~46K lines). We implement the core loop, not everything.

```rust
// crates/c4l-engine/src/lib.rs

use c4l_api::{AnthropicClient, StreamEvent, ContentBlock};
use c4l_types::{Message, ToolUseContext};
use tokio::sync::mpsc;

/// The core conversation engine
/// Maps from: QueryEngine class in TypeEngine.ts
pub struct QueryEngine {
    client: AnthropicClient,
    config: EngineConfig,
    tool_registry: ToolRegistry,
    state: EngineState,
}

pub struct EngineConfig {
    pub max_turns: u32,              // default: 100
    pub max_tokens_per_turn: u32,    // default: 16384
    pub thinking_budget: Option<u32>,
    pub system_prompt: String,
    pub append_system_prompt: Option<String>,
}

struct EngineState {
    messages: Vec<Message>,
    total_usage: UsageAccumulator,
    turn_count: u32,
}

/// Events emitted during a query
/// Maps from: stream_submit_message() generator in query_engine.py
#[derive(Debug, Clone)]
pub enum QueryEvent {
    /// Text content streaming
    TextDelta(String),
    /// Thinking content streaming
    ThinkingDelta(String),
    /// Tool use detected — engine will execute
    ToolUseStart { id: String, name: String },
    /// Tool execution result
    ToolResult { id: String, result: ToolExecutionResult },
    /// Usage update
    Usage(UsageData),
    /// Turn completed
    TurnComplete { stop_reason: StopReason },
    /// Error (may retry)
    Error(String),
}

#[derive(Debug, Clone)]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,       // more tool calls to process
    StopSequence,
}

impl QueryEngine {
    pub fn new(
        client: AnthropicClient,
        config: EngineConfig,
        tool_registry: ToolRegistry,
    ) -> Self;

    /// Submit a user message and run the full tool-call loop
    /// This is THE core function — maps from QueryEngine.ts main loop
    ///
    /// Flow:
    /// 1. Append user message to history
    /// 2. Build API request (system prompt + messages + tools)
    /// 3. Stream response from API
    /// 4. Accumulate content blocks
    /// 5. If tool_use blocks found:
    ///    a. Execute each tool (via ToolRegistry)
    ///    b. Append tool results as user message
    ///    c. Go to step 2 (next turn)
    /// 6. If no tool_use: conversation turn complete
    /// 7. Return final assistant message
    pub async fn submit(
        &mut self,
        user_message: String,
        event_tx: mpsc::Sender<QueryEvent>,
    ) -> Result<Message>;

    /// Build the messages array for the API request
    /// Maps from: normalizeMessagesForAPI() in QueryEngine.ts
    fn build_api_messages(&self) -> Vec<ApiMessage>;

    /// Build the tools array for the API request
    fn build_api_tools(&self) -> Vec<ApiToolDef>;

    /// Execute a single tool call
    /// Maps from: tool execution in QueryEngine.ts tool-call loop
    async fn execute_tool(
        &self,
        tool_name: &str,
        tool_input: serde_json::Value,
        tool_use_id: &str,
        event_tx: &mpsc::Sender<QueryEvent>,
    ) -> Result<ToolExecutionResult>;
}

/// Manages accumulated token usage across turns
struct UsageAccumulator {
    total_input: u64,
    total_output: u64,
    total_cache_creation: u64,
    total_cache_read: u64,
}
```

### Key Design Decisions

**Why mpsc channel for events (not callbacks):**
- Rust ownership model makes callback chains painful
- Channel decouples engine from UI rendering
- TUI can consume events at its own pace
- Easy to add event logging/recording

**Why not port QueryEngine.ts line-by-line:**
- 46K lines includes React UI rendering, plugin hooks, feature flags, telemetry
- Core loop is ~2K lines of actual logic
- Rest is integration code we build incrementally in later phases

**Message normalization:**
- TypeScript strips UI-only metadata before API calls
- We keep messages clean from the start (no UI metadata in Message type)
- Separate UI state from conversation state

## Deliverables for Phase 1

1. `c4l-api` crate with streaming Anthropic client
2. `c4l-engine` crate with tool-call loop
3. Integration test: send message → get streamed response (mock server)
4. Integration test: tool_use → execute → tool_result → continue (mock)
5. Basic error handling with retry policy
6. `cargo test --workspace` all passing
