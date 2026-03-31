//! Rendering functions for the TUI layout.
//!
//! Each function renders a specific area of the screen into a ratatui Frame.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::AppMode;
use crate::input::InputState;

/// Render the header bar.
pub fn render_header(frame: &mut Frame, area: Rect, model: &str, session_id: &str) {
    let text = format!(" claw4love | {} | {}", model, &session_id[..8.min(session_id.len())]);
    let header = Paragraph::new(text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(header, area);
}

/// Render the message list area.
pub fn render_messages(
    frame: &mut Frame,
    area: Rect,
    messages: &[DisplayMessage],
    scroll_offset: usize,
    streaming_text: Option<&str>,
) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in messages {
        match msg {
            DisplayMessage::User(text) => {
                lines.push(Line::from(""));
                lines.push(
                    Line::from(vec![
                        Span::styled("User: ", Style::default().fg(Color::Cyan).bold()),
                        Span::raw(text),
                    ])
                );
            }
            DisplayMessage::Assistant(text) => {
                lines.push(Line::from(""));
                lines.push(
                    Line::from(Span::styled("Assistant:", Style::default().fg(Color::Green).bold()))
                );
                for line in text.lines() {
                    lines.push(Line::from(format!("  {line}")));
                }
            }
            DisplayMessage::ToolUse { name, input_summary } => {
                lines.push(
                    Line::from(vec![
                        Span::styled("  [Tool: ", Style::default().fg(Color::Yellow)),
                        Span::styled(name, Style::default().fg(Color::Yellow).bold()),
                        Span::styled("] ", Style::default().fg(Color::Yellow)),
                        Span::styled(input_summary, Style::default().fg(Color::DarkGray)),
                    ])
                );
            }
            DisplayMessage::ToolResult { name, summary, is_error } => {
                let color = if *is_error { Color::Red } else { Color::Green };
                let prefix = if *is_error { "ERR" } else { "OK" };
                lines.push(
                    Line::from(vec![
                        Span::styled(format!("  [{prefix}: {name}] "), Style::default().fg(color)),
                        Span::raw(summary),
                    ])
                );
            }
            DisplayMessage::System(text) => {
                lines.push(
                    Line::from(Span::styled(format!("  {text}"), Style::default().fg(Color::DarkGray)))
                );
            }
        }
    }

    // Append streaming text if present
    if let Some(streaming) = streaming_text {
        if !streaming.is_empty() {
            lines.push(Line::from(""));
            lines.push(
                Line::from(Span::styled("Assistant:", Style::default().fg(Color::Green).bold()))
            );
            for line in streaming.lines() {
                lines.push(Line::from(format!("  {line}")));
            }
            lines.push(Line::from(Span::styled("...", Style::default().fg(Color::DarkGray))));
        }
    }

    let total_lines = lines.len() as u16;
    let visible_height = area.height;

    // Auto-scroll to bottom unless user has scrolled up
    let scroll = if scroll_offset == 0 {
        total_lines.saturating_sub(visible_height)
    } else {
        scroll_offset as u16
    };

    let messages_widget = Paragraph::new(lines)
        .scroll((scroll, 0))
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(messages_widget, area);
}

/// Render the input area.
pub fn render_input(frame: &mut Frame, area: Rect, input: &InputState, mode: &AppMode) {
    let prompt = match mode {
        AppMode::Streaming => " ... ",
        AppMode::PermissionPrompt { .. } => " [y/n] ",
        _ => " > ",
    };

    let display_text = if matches!(mode, AppMode::Streaming) {
        String::from("(streaming...)")
    } else {
        input.content()
    };

    let input_widget = Paragraph::new(format!("{prompt}{display_text}"))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(input_widget, area);

    // Show cursor position
    if matches!(mode, AppMode::Input) {
        let cursor_x = area.x + prompt.len() as u16 + input.cursor_col as u16;
        let cursor_y = area.y + 1 + input.cursor_row as u16; // +1 for border
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Render the status bar.
pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
    cost_usd: f64,
    elapsed: std::time::Duration,
) {
    let elapsed_str = if elapsed.as_secs() >= 60 {
        format!("{}m{}s", elapsed.as_secs() / 60, elapsed.as_secs() % 60)
    } else {
        format!("{}s", elapsed.as_secs())
    };

    let tokens_str = format_tokens(input_tokens + output_tokens);
    let cost_str = if cost_usd >= 0.01 {
        format!("${cost_usd:.2}")
    } else if cost_usd > 0.0 {
        format!("${cost_usd:.4}")
    } else {
        "$0.00".into()
    };

    let text = format!(
        " {model} | {tokens_str} tokens | {cost_str} | {elapsed_str}"
    );

    let status = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Black));
    frame.render_widget(status, area);
}

/// Render a permission prompt overlay.
pub fn render_permission_prompt(
    frame: &mut Frame,
    area: Rect,
    tool_name: &str,
    description: &str,
) {
    let block = Block::default()
        .title(format!(" Allow {tool_name}? "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let text = vec![
        Line::from(""),
        Line::from(format!("  {description}")),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y] ", Style::default().fg(Color::Green).bold()),
            Span::raw("Allow  "),
            Span::styled("[a] ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Always  "),
            Span::styled("[n] ", Style::default().fg(Color::Red).bold()),
            Span::raw("Deny"),
        ]),
        Line::from(""),
    ];

    let widget = Paragraph::new(text).block(block);

    // Center the dialog
    let popup_area = centered_rect(60, 7, area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(widget, popup_area);
}

/// Helper: format token count with K/M suffixes.
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Helper: compute a centered rectangle.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let width = (area.width * percent_x / 100).min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height.min(area.height))
}

/// A display-ready message for rendering (simplified from Message enum).
#[derive(Debug, Clone)]
pub enum DisplayMessage {
    User(String),
    Assistant(String),
    ToolUse {
        name: String,
        input_summary: String,
    },
    ToolResult {
        name: String,
        summary: String,
        is_error: bool,
    },
    System(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tokens_values() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1500), "1.5K");
        assert_eq!(format_tokens(42000), "42.0K");
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn centered_rect_computation() {
        let area = Rect::new(0, 0, 100, 40);
        let popup = centered_rect(60, 7, area);
        assert_eq!(popup.width, 60);
        assert_eq!(popup.height, 7);
        assert_eq!(popup.x, 20); // centered
    }
}
