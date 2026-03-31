//! Server-Sent Events (SSE) parser for Anthropic streaming API.
//!
//! The Anthropic API sends events in SSE format:
//!   event: message_start
//!   data: {"type":"message_start","message":{...}}
//!
//!   event: content_block_delta
//!   data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}

use crate::error::ApiError;
use crate::types::StreamEvent;

/// Parse a single SSE data line into a StreamEvent.
///
/// Input is the raw `data:` payload (without the "data: " prefix).
/// Returns None for empty lines or "[DONE]" signals.
pub fn parse_sse_data(data: &str) -> Result<Option<StreamEvent>, ApiError> {
    let data = data.trim();

    if data.is_empty() || data == "[DONE]" {
        return Ok(None);
    }

    serde_json::from_str::<StreamEvent>(data)
        .map(Some)
        .map_err(|e| ApiError::SseParse(format!("failed to parse SSE data: {e}\nraw: {data}")))
}

/// Iterator over SSE events from a raw byte stream.
///
/// Handles the SSE wire format:
/// - Lines starting with "data: " contain JSON payloads
/// - Lines starting with "event: " indicate event type (we use the type field in JSON instead)
/// - Empty lines separate events
/// - Lines starting with ":" are comments (ignored)
pub struct SseLineParser {
    buffer: String,
}

impl SseLineParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed a chunk of bytes from the HTTP response body.
    /// Returns any complete events parsed from the accumulated buffer.
    pub fn feed(&mut self, chunk: &str) -> Vec<Result<StreamEvent, ApiError>> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        // Process complete lines
        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].trim_end_matches('\r').to_string();
            self.buffer = self.buffer[newline_pos + 1..].to_string();

            if let Some(data) = line.strip_prefix("data: ") {
                match parse_sse_data(data) {
                    Ok(Some(event)) => events.push(Ok(event)),
                    Ok(None) => {} // empty or [DONE]
                    Err(e) => events.push(Err(e)),
                }
            }
            // Skip "event:", ":", and empty lines
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContentDelta, ResponseContentBlock};

    #[test]
    fn parse_text_delta() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let event = parse_sse_data(data).unwrap().unwrap();
        match event {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                assert!(matches!(delta, ContentDelta::TextDelta { text } if text == "Hello"));
            }
            _ => panic!("expected content_block_delta"),
        }
    }

    #[test]
    fn parse_message_stop() {
        let data = r#"{"type":"message_stop"}"#;
        let event = parse_sse_data(data).unwrap().unwrap();
        assert!(matches!(event, StreamEvent::MessageStop {}));
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(parse_sse_data("").unwrap().is_none());
        assert!(parse_sse_data("[DONE]").unwrap().is_none());
    }

    #[test]
    fn parse_ping() {
        let data = r#"{"type":"ping"}"#;
        let event = parse_sse_data(data).unwrap().unwrap();
        assert!(matches!(event, StreamEvent::Ping {}));
    }

    #[test]
    fn line_parser_multiple_events() {
        let mut parser = SseLineParser::new();

        let chunk = "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n";

        let events = parser.feed(chunk);
        assert_eq!(events.len(), 2);

        match events[0].as_ref().unwrap() {
            StreamEvent::ContentBlockStart { index, content_block } => {
                assert_eq!(*index, 0);
                assert!(matches!(content_block, ResponseContentBlock::Text { .. }));
            }
            _ => panic!("expected content_block_start"),
        }

        match events[1].as_ref().unwrap() {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                assert!(matches!(delta, ContentDelta::TextDelta { text } if text == "Hi"));
            }
            _ => panic!("expected content_block_delta"),
        }
    }

    #[test]
    fn line_parser_partial_chunks() {
        let mut parser = SseLineParser::new();

        // Feed partial data
        let events1 = parser.feed("data: {\"type\":\"pi");
        assert_eq!(events1.len(), 0); // no complete line yet

        // Complete the line
        let events2 = parser.feed("ng\"}\n\n");
        assert_eq!(events2.len(), 1);
        assert!(matches!(events2[0].as_ref().unwrap(), StreamEvent::Ping {}));
    }
}
