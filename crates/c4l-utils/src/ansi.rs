//! ANSI escape code stripping.
//!
//! Pattern from: RTK core/utils.rs strip_ansi()

use regex::Regex;
use std::sync::LazyLock;

static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\].*?\x07").unwrap());

/// Strip ANSI escape codes from text.
pub fn strip_ansi(text: &str) -> String {
    ANSI_RE.replace_all(text, "").to_string()
}

/// Truncate text to max characters, appending "..." if truncated.
pub fn truncate(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        format!("{}...", &text[..max_chars])
    }
}

/// Truncate to max lines, appending a note if truncated.
pub fn truncate_lines(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        text.to_string()
    } else {
        let kept: Vec<&str> = lines[..max_lines].to_vec();
        format!(
            "{}\n\n... ({} more lines)",
            kept.join("\n"),
            lines.len() - max_lines
        )
    }
}

/// Strip trailing whitespace from each line.
pub fn strip_trailing_whitespace(text: &str) -> String {
    text.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Remove consecutive duplicate blank lines.
pub fn deduplicate_blank_lines(text: &str) -> String {
    let mut result = String::new();
    let mut prev_blank = false;

    for line in text.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
        prev_blank = is_blank;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_codes() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi("\x1b[1;32mbold green\x1b[0m"), "bold green");
        assert_eq!(strip_ansi("no codes here"), "no codes here");
    }

    #[test]
    fn truncate_text() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn truncate_lines_works() {
        let text = "a\nb\nc\nd\ne";
        let result = truncate_lines(text, 3);
        assert!(result.contains("a\nb\nc"));
        assert!(result.contains("2 more lines"));
    }

    #[test]
    fn dedup_blank_lines() {
        let text = "a\n\n\n\nb\n\nc";
        let result = deduplicate_blank_lines(text);
        assert_eq!(result, "a\n\nb\n\nc");
    }
}
