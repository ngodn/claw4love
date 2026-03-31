//! The core query engine: conversation loop with tool execution.
//!
//! Maps from: leak-claude-code/src/QueryEngine.ts main loop
//!
//! Flow:
//! 1. Receive user message
//! 2. Build API request (system prompt + messages + tool definitions)
//! 3. Stream response from Anthropic API
//! 4. Accumulate content blocks
//! 5. If tool_use blocks found: execute each tool, append results, go to step 2
//! 6. If no tool_use: turn complete, return

use crate::events::{QueryEvent, StopReason};
use crate::tool_registry::{ToolExecResult, ToolRegistry};
use c4l_api::{
    AnthropicClient, ApiContent, ApiContentBlock, ApiMessage, ContentDelta, ResponseContentBlock,
    StreamEvent, UsageData,
};
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Configuration for the query engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Maximum number of tool-call loop turns before stopping.
    pub max_turns: u32,
    /// System prompt sent with every request.
    pub system_prompt: String,
    /// Additional text appended to the system prompt.
    pub append_system_prompt: Option<String>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_turns: 100,
            system_prompt: String::new(),
            append_system_prompt: None,
        }
    }
}

/// The conversation engine.
pub struct QueryEngine {
    client: AnthropicClient,
    config: EngineConfig,
    tool_registry: ToolRegistry,
    messages: Vec<ApiMessage>,
    total_usage: UsageAccumulator,
}

#[derive(Debug, Default)]
struct UsageAccumulator {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation: u64,
    cache_read: u64,
}

impl UsageAccumulator {
    fn add(&mut self, usage: &UsageData) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.cache_creation += usage.cache_creation_input_tokens.unwrap_or(0);
        self.cache_read += usage.cache_read_input_tokens.unwrap_or(0);
    }

    fn to_usage_data(&self) -> UsageData {
        UsageData {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cache_creation_input_tokens: Some(self.cache_creation),
            cache_read_input_tokens: Some(self.cache_read),
        }
    }
}

impl QueryEngine {
    pub fn new(client: AnthropicClient, config: EngineConfig, tool_registry: ToolRegistry) -> Self {
        Self {
            client,
            config,
            tool_registry,
            messages: Vec::new(),
            total_usage: UsageAccumulator::default(),
        }
    }

    /// Get the full system prompt (base + append).
    fn system_prompt(&self) -> String {
        let mut prompt = self.config.system_prompt.clone();
        if let Some(append) = &self.config.append_system_prompt {
            prompt.push_str("\n\n");
            prompt.push_str(append);
        }
        prompt
    }

    /// Submit a user message and run the full tool-call loop.
    ///
    /// Events are sent through the channel as they happen.
    /// Returns when the conversation turn is complete (no more tool calls).
    pub async fn submit(
        &mut self,
        user_message: String,
        event_tx: mpsc::Sender<QueryEvent>,
    ) -> Result<()> {
        // Append user message to history
        self.messages.push(ApiMessage {
            role: "user".into(),
            content: ApiContent::Text(user_message),
        });

        let system = self.system_prompt();
        let tool_defs = self.tool_registry.api_tool_defs();

        let mut turn = 0u32;

        loop {
            if turn >= self.config.max_turns {
                warn!(turn, "max turns reached, stopping");
                let _ = event_tx
                    .send(QueryEvent::TurnComplete {
                        stop_reason: StopReason::MaxTokens,
                    })
                    .await;
                break;
            }
            turn += 1;
            debug!(turn, messages = self.messages.len(), "starting turn");

            // Stream the API response and process events inline.
            // We use a channel between the HTTP stream reader and event processor
            // so the engine can be extended to run them concurrently later.
            let (stream_tx, mut stream_rx) = mpsc::channel::<Result<StreamEvent, c4l_api::ApiError>>(256);

            // Run streaming in a background task
            let msgs = self.messages.clone();
            let sys = system.clone();
            let tools = tool_defs.clone();
            // Stream messages inline. The sender is moved into stream_messages,
            // which drops it when done, closing the channel.
            self.client
                .stream_messages(&msgs, Some(&sys), &tools, stream_tx)
                .await
                .ok(); // errors are sent through the channel

            // Process stream events
            let mut content_blocks: Vec<ResponseContentBlock> = Vec::new();
            let mut stop_reason = StopReason::EndTurn;
            let mut tool_inputs: std::collections::HashMap<usize, String> = std::collections::HashMap::new();

            while let Some(event_result) = stream_rx.recv().await {
                match event_result {
                    Ok(event) => match event {
                        StreamEvent::ContentBlockStart {
                            index,
                            content_block,
                        } => {
                            // Ensure we have enough slots
                            while content_blocks.len() <= index {
                                content_blocks.push(ResponseContentBlock::Text {
                                    text: String::new(),
                                });
                            }
                            content_blocks[index] = content_block.clone();

                            if let ResponseContentBlock::ToolUse { id, name, .. } = &content_block {
                                let _ = event_tx
                                    .send(QueryEvent::ToolUseStart {
                                        id: id.clone(),
                                        name: name.clone(),
                                    })
                                    .await;
                            }
                        }
                        StreamEvent::ContentBlockDelta { index, delta } => match &delta {
                            ContentDelta::TextDelta { text } => {
                                // Update accumulated text
                                if let Some(ResponseContentBlock::Text { text: existing }) =
                                    content_blocks.get_mut(index)
                                {
                                    existing.push_str(text);
                                }
                                let _ = event_tx.send(QueryEvent::TextDelta(text.clone())).await;
                            }
                            ContentDelta::ThinkingDelta { thinking } => {
                                let _ = event_tx
                                    .send(QueryEvent::ThinkingDelta(thinking.clone()))
                                    .await;
                            }
                            ContentDelta::InputJsonDelta { partial_json } => {
                                tool_inputs
                                    .entry(index)
                                    .or_default()
                                    .push_str(partial_json);
                                let _ = event_tx
                                    .send(QueryEvent::ToolInputDelta {
                                        id: format!("block_{index}"),
                                        partial_json: partial_json.clone(),
                                    })
                                    .await;
                            }
                            // Signature, citations, connector deltas: skip for now
                            _ => {}
                        },
                        StreamEvent::ContentBlockStop { index } => {
                            // Finalize tool_use input from accumulated JSON
                            if let Some(json_str) = tool_inputs.remove(&index) {
                                if let Some(ResponseContentBlock::ToolUse { input, .. }) =
                                    content_blocks.get_mut(index)
                                {
                                    if let Ok(parsed) = serde_json::from_str(&json_str) {
                                        *input = parsed;
                                    }
                                }
                            }
                        }
                        StreamEvent::MessageDelta { delta, usage } => {
                            if let Some(reason) = &delta.stop_reason {
                                stop_reason = StopReason::from_api(reason);
                            }
                            self.total_usage.add(&usage);
                            let _ = event_tx.send(QueryEvent::Usage(usage)).await;
                        }
                        StreamEvent::MessageStop {} => {}
                        StreamEvent::Ping {} => {}
                        StreamEvent::Error { error } => {
                            let _ = event_tx
                                .send(QueryEvent::Error(format!(
                                    "{}: {}",
                                    error.error_type, error.message
                                )))
                                .await;
                        }
                        _ => {}
                    },
                    Err(e) => {
                        let _ = event_tx.send(QueryEvent::Error(e.to_string())).await;
                    }
                }
            }

            // Append assistant message to history
            self.messages.push(ApiMessage {
                role: "assistant".into(),
                content: ApiContent::Blocks(
                    content_blocks
                        .iter()
                        .map(|b| match b {
                            ResponseContentBlock::Text { text } => {
                                ApiContentBlock::Text { text: text.clone() }
                            }
                            ResponseContentBlock::Thinking { .. }
                            | ResponseContentBlock::ServerToolUse { .. } => {
                                // Thinking and server tool blocks are not sent back
                                ApiContentBlock::Text {
                                    text: String::new(),
                                }
                            }
                            ResponseContentBlock::ToolUse { id, name, input } => {
                                ApiContentBlock::ToolUse {
                                    id: id.clone(),
                                    name: name.clone(),
                                    input: input.clone(),
                                }
                            }
                        })
                        .collect(),
                ),
            });

            // If stop reason is tool_use, execute tools and continue
            if stop_reason.should_continue() {
                let tool_uses: Vec<_> = content_blocks
                    .iter()
                    .filter_map(|b| match b {
                        ResponseContentBlock::ToolUse { id, name, input } => {
                            Some((id.clone(), name.clone(), input.clone()))
                        }
                        _ => None,
                    })
                    .collect();

                info!(count = tool_uses.len(), "executing tool calls");

                let mut tool_results: Vec<ApiContentBlock> = Vec::new();

                for (id, name, input) in tool_uses {
                    let exec_result = self.tool_registry.execute(&name, input.clone()).await;

                    let (content, is_error) = match exec_result {
                        Ok(ToolExecResult { content, is_error }) => (content, is_error),
                        Err(e) => (
                            serde_json::Value::String(format!("Tool execution error: {e}")),
                            true,
                        ),
                    };

                    let _ = event_tx
                        .send(QueryEvent::ToolResult {
                            id: id.clone(),
                            name: name.clone(),
                            result: content.clone(),
                            is_error,
                        })
                        .await;

                    tool_results.push(ApiContentBlock::ToolResult {
                        tool_use_id: id,
                        content,
                        is_error: if is_error { Some(true) } else { None },
                    });
                }

                // Append tool results as user message
                self.messages.push(ApiMessage {
                    role: "user".into(),
                    content: ApiContent::Blocks(tool_results),
                });

                // Continue the loop for the next turn
                continue;
            }

            // No more tool calls, conversation turn is done
            let _ = event_tx
                .send(QueryEvent::TurnComplete { stop_reason })
                .await;
            break;
        }

        Ok(())
    }

    /// Get the accumulated token usage.
    pub fn total_usage(&self) -> UsageData {
        self.total_usage.to_usage_data()
    }

    /// Get the current conversation messages.
    pub fn messages(&self) -> &[ApiMessage] {
        &self.messages
    }

    /// Clear conversation history (for /clear command).
    pub fn clear(&mut self) {
        self.messages.clear();
        self.total_usage = UsageAccumulator::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn engine_config_defaults() {
        let config = EngineConfig::default();
        assert_eq!(config.max_turns, 100);
        assert!(config.system_prompt.is_empty());
    }

    #[test]
    fn system_prompt_concatenation() {
        let config = EngineConfig {
            system_prompt: "Base prompt.".into(),
            append_system_prompt: Some("Extra context.".into()),
            ..Default::default()
        };

        let client = AnthropicClient::new(c4l_api::ApiConfig::new(
            "test".into(),
            "test".into(),
        ));
        let engine = QueryEngine::new(client, config, ToolRegistry::new());

        let prompt = engine.system_prompt();
        assert!(prompt.contains("Base prompt."));
        assert!(prompt.contains("Extra context."));
    }

    #[test]
    fn engine_starts_with_empty_history() {
        let client = AnthropicClient::new(c4l_api::ApiConfig::new(
            "test".into(),
            "test".into(),
        ));
        let engine = QueryEngine::new(client, EngineConfig::default(), ToolRegistry::new());

        assert!(engine.messages().is_empty());
        assert_eq!(engine.total_usage().input_tokens, 0);
    }
}
