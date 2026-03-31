//! Token optimization filter pipeline.
//!
//! Ported from: RTK core/toml_filter.rs (600+ lines)
//! This is the KEY IMPROVEMENT over vanilla Claude Code.
//!
//! 8-stage pipeline:
//! 1. strip_ansi — remove ANSI escape codes
//! 2. replace — regex substitutions (line-by-line)
//! 3. match_output — short-circuit on known patterns
//! 4. strip/keep_lines — filter lines by regex
//! 5. truncate_lines_at — truncate each line to N chars
//! 6. head/tail_lines — keep first/last N lines
//! 7. max_lines — absolute line cap
//! 8. on_empty — message if result empty

use regex::Regex;
use crate::ansi;

/// A compiled filter ready for execution.
#[derive(Debug)]
pub struct CompiledFilter {
    pub name: String,
    pub description: Option<String>,
    pub match_command: Regex,
    pub strip_ansi: bool,
    pub replace_rules: Vec<(Regex, String)>,
    pub match_output: Vec<(Regex, String)>,
    pub keep_lines: Option<Regex>,
    pub strip_lines: Option<Regex>,
    pub truncate_lines_at: Option<usize>,
    pub head_lines: Option<usize>,
    pub tail_lines: Option<usize>,
    pub max_lines: Option<usize>,
    pub on_empty: Option<String>,
}

/// Result of applying a filter.
/// RTK's 3-tier pattern: Full / Partial / Passthrough.
#[derive(Debug, Clone)]
pub enum FilterResult {
    /// Filter matched and produced clean output.
    Full(FilteredOutput),
    /// Filter matched but had warnings.
    Partial(FilteredOutput, Vec<String>),
    /// No filter matched; returning truncated raw output.
    Passthrough(String),
}

#[derive(Debug, Clone)]
pub struct FilteredOutput {
    pub content: String,
    pub original_len: usize,
    pub filtered_len: usize,
    pub savings_pct: f64,
}

/// The filter pipeline holding all compiled filters.
pub struct FilterPipeline {
    filters: Vec<CompiledFilter>,
}

impl FilterPipeline {
    pub fn new() -> Self {
        Self { filters: Vec::new() }
    }

    pub fn add(&mut self, filter: CompiledFilter) {
        self.filters.push(filter);
    }

    pub fn len(&self) -> usize {
        self.filters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// Apply the pipeline to command output.
    /// Finds the first matching filter and runs it.
    pub fn apply(&self, command: &str, output: &str) -> FilterResult {
        for filter in &self.filters {
            if filter.match_command.is_match(command) {
                return apply_filter(filter, output);
            }
        }

        // No filter matched: passthrough with truncation
        let truncated = ansi::truncate_lines(output, 200);
        FilterResult::Passthrough(truncated)
    }
}

impl Default for FilterPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply a single filter to output text.
fn apply_filter(filter: &CompiledFilter, output: &str) -> FilterResult {
    let original_len = output.len();
    let mut text = output.to_string();

    // Stage 1: strip ANSI
    if filter.strip_ansi {
        text = ansi::strip_ansi(&text);
    }

    // Stage 2: regex replacements
    for (pattern, replacement) in &filter.replace_rules {
        text = pattern.replace_all(&text, replacement.as_str()).to_string();
    }

    // Stage 3: match_output short-circuit
    for (pattern, message) in &filter.match_output {
        if pattern.is_match(&text) {
            let result = FilteredOutput {
                content: message.clone(),
                original_len,
                filtered_len: message.len(),
                savings_pct: savings_pct(original_len, message.len()),
            };
            return FilterResult::Full(result);
        }
    }

    // Stage 4: line filtering
    let lines: Vec<&str> = text.lines().collect();
    let lines = if let Some(keep) = &filter.keep_lines {
        lines.into_iter().filter(|l| keep.is_match(l)).collect()
    } else if let Some(strip) = &filter.strip_lines {
        lines.into_iter().filter(|l| !strip.is_match(l)).collect()
    } else {
        lines
    };

    // Stage 5: truncate each line
    let lines: Vec<String> = if let Some(max_width) = filter.truncate_lines_at {
        lines.into_iter().map(|l| ansi::truncate(l, max_width)).collect()
    } else {
        lines.into_iter().map(String::from).collect()
    };

    // Stage 6: head/tail
    let lines = if let Some(head) = filter.head_lines {
        lines[..head.min(lines.len())].to_vec()
    } else if let Some(tail) = filter.tail_lines {
        let start = lines.len().saturating_sub(tail);
        lines[start..].to_vec()
    } else {
        lines
    };

    // Stage 7: max lines
    let lines = if let Some(max) = filter.max_lines {
        if lines.len() > max {
            let mut truncated = lines[..max].to_vec();
            truncated.push(format!("... ({} more lines)", lines.len() - max));
            truncated
        } else {
            lines
        }
    } else {
        lines
    };

    // Stage 8: on_empty
    let result_text = lines.join("\n");
    let final_text = if result_text.trim().is_empty() {
        filter.on_empty.clone().unwrap_or_default()
    } else {
        result_text
    };

    let filtered_len = final_text.len();
    FilterResult::Full(FilteredOutput {
        content: final_text,
        original_len,
        filtered_len,
        savings_pct: savings_pct(original_len, filtered_len),
    })
}

fn savings_pct(original: usize, filtered: usize) -> f64 {
    if original == 0 {
        return 0.0;
    }
    (1.0 - (filtered as f64 / original as f64)) * 100.0
}

/// Parse a TOML filter definition into a CompiledFilter.
pub fn compile_filter(name: &str, table: &toml::Value) -> anyhow::Result<CompiledFilter> {
    let get_str = |key: &str| table.get(key).and_then(|v| v.as_str());
    let get_bool = |key: &str| table.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
    let get_usize = |key: &str| table.get(key).and_then(|v| v.as_integer()).map(|i| i as usize);

    let match_command = get_str("match_command")
        .ok_or_else(|| anyhow::anyhow!("filter '{name}' missing match_command"))?;

    let mut replace_rules = Vec::new();
    if let Some(rules) = table.get("replace").and_then(|v| v.as_array()) {
        for rule in rules {
            if let (Some(pattern), Some(replacement)) = (
                rule.get("pattern").and_then(|v| v.as_str()),
                rule.get("replacement").and_then(|v| v.as_str()),
            ) {
                if let Ok(re) = Regex::new(pattern) {
                    replace_rules.push((re, replacement.to_string()));
                }
            }
        }
    }

    let mut match_output = Vec::new();
    if let Some(rules) = table.get("match_output").and_then(|v| v.as_array()) {
        for rule in rules {
            if let (Some(pattern), Some(message)) = (
                rule.get("pattern").and_then(|v| v.as_str()),
                rule.get("message").and_then(|v| v.as_str()),
            ) {
                if let Ok(re) = Regex::new(pattern) {
                    match_output.push((re, message.to_string()));
                }
            }
        }
    }

    Ok(CompiledFilter {
        name: name.into(),
        description: get_str("description").map(String::from),
        match_command: Regex::new(match_command)?,
        strip_ansi: get_bool("strip_ansi"),
        replace_rules,
        match_output,
        keep_lines: get_str("keep_lines_matching").and_then(|s| Regex::new(s).ok()),
        strip_lines: get_str("strip_lines_matching").and_then(|s| Regex::new(s).ok()),
        truncate_lines_at: get_usize("truncate_lines_at"),
        head_lines: get_usize("head_lines"),
        tail_lines: get_usize("tail_lines"),
        max_lines: get_usize("max_lines"),
        on_empty: get_str("on_empty").map(String::from),
    })
}

/// Load filters from a TOML string.
pub fn load_filters_from_toml(toml_str: &str) -> anyhow::Result<Vec<CompiledFilter>> {
    let value: toml::Value = toml_str.parse()?;
    let mut filters = Vec::new();

    if let Some(table) = value.get("filters").and_then(|v| v.as_table()) {
        for (name, filter_table) in table {
            match compile_filter(name, filter_table) {
                Ok(f) => filters.push(f),
                Err(e) => tracing::warn!(name, %e, "failed to compile filter"),
            }
        }
    }

    Ok(filters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn savings_calculation() {
        assert!((savings_pct(1000, 200) - 80.0).abs() < 0.1);
        assert!((savings_pct(100, 100) - 0.0).abs() < 0.1);
        assert!((savings_pct(0, 0) - 0.0).abs() < 0.1);
    }

    #[test]
    fn compile_basic_filter() {
        let toml_str = r#"
[filters.cargo-test]
description = "Compact cargo test output"
match_command = "^cargo test"
strip_ansi = true
max_lines = 50
on_empty = "All tests passed"

[[filters.cargo-test.match_output]]
pattern = "test result: ok"
message = "All tests passed"
"#;
        let filters = load_filters_from_toml(toml_str).unwrap();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].name, "cargo-test");
        assert!(filters[0].strip_ansi);
        assert_eq!(filters[0].max_lines, Some(50));
        assert_eq!(filters[0].match_output.len(), 1);
    }

    #[test]
    fn pipeline_matches_and_filters() {
        let filter = CompiledFilter {
            name: "test".into(),
            description: None,
            match_command: Regex::new("^echo").unwrap(),
            strip_ansi: false,
            replace_rules: vec![],
            match_output: vec![],
            keep_lines: None,
            strip_lines: None,
            truncate_lines_at: None,
            head_lines: Some(3),
            tail_lines: None,
            max_lines: None,
            on_empty: None,
        };

        let mut pipeline = FilterPipeline::new();
        pipeline.add(filter);

        let result = pipeline.apply("echo hello", "line1\nline2\nline3\nline4\nline5");
        match result {
            FilterResult::Full(output) => {
                assert_eq!(output.content, "line1\nline2\nline3");
                assert!(output.savings_pct > 0.0);
            }
            _ => panic!("expected Full"),
        }
    }

    #[test]
    fn pipeline_passthrough_on_no_match() {
        let pipeline = FilterPipeline::new();
        let result = pipeline.apply("unknown-command", "some output");
        assert!(matches!(result, FilterResult::Passthrough(_)));
    }

    #[test]
    fn match_output_short_circuits() {
        let filter = CompiledFilter {
            name: "test".into(),
            description: None,
            match_command: Regex::new("^cargo test").unwrap(),
            strip_ansi: false,
            replace_rules: vec![],
            match_output: vec![(Regex::new("test result: ok").unwrap(), "All tests passed".into())],
            keep_lines: None,
            strip_lines: None,
            truncate_lines_at: None,
            head_lines: None,
            tail_lines: None,
            max_lines: None,
            on_empty: None,
        };

        let mut pipeline = FilterPipeline::new();
        pipeline.add(filter);

        let output = "running 10 tests\ntest foo ... ok\ntest bar ... ok\ntest result: ok. 10 passed\n";
        let result = pipeline.apply("cargo test", output);
        match result {
            FilterResult::Full(fo) => {
                assert_eq!(fo.content, "All tests passed");
                assert!(fo.savings_pct > 50.0);
            }
            _ => panic!("expected Full with short-circuit"),
        }
    }

    #[test]
    fn keep_lines_filters() {
        let filter = CompiledFilter {
            name: "test".into(),
            description: None,
            match_command: Regex::new("^cargo").unwrap(),
            strip_ansi: false,
            replace_rules: vec![],
            match_output: vec![],
            keep_lines: Some(Regex::new("FAILED|error").unwrap()),
            strip_lines: None,
            truncate_lines_at: None,
            head_lines: None,
            tail_lines: None,
            max_lines: None,
            on_empty: Some("No failures".into()),
        };

        let mut pipeline = FilterPipeline::new();
        pipeline.add(filter);

        // With failures
        let result = pipeline.apply("cargo test", "test foo ... ok\ntest bar ... FAILED\nerror: assertion\ntest baz ... ok");
        match result {
            FilterResult::Full(fo) => {
                assert!(fo.content.contains("FAILED"));
                assert!(fo.content.contains("error"));
                assert!(!fo.content.contains("ok"));
            }
            _ => panic!("expected Full"),
        }

        // All passing
        let result = pipeline.apply("cargo test", "test foo ... ok\ntest bar ... ok");
        match result {
            FilterResult::Full(fo) => assert_eq!(fo.content, "No failures"),
            _ => panic!("expected Full"),
        }
    }
}
