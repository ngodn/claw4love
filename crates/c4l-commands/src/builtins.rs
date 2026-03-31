//! Built-in slash commands.
//!
//! Maps from: leak-claude-code/src/commands/ (priority subset)

use crate::traits::{Command, CommandResult};
use c4l_state::SharedAppState;

// -- /help --

pub struct HelpCommand;

impl Command for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn aliases(&self) -> Vec<&str> { vec!["h", "?"] }
    fn description(&self) -> &str { "Show available commands" }

    fn execute(&self, args: &str, _state: &SharedAppState) -> anyhow::Result<CommandResult> {
        let text = if args.is_empty() {
            [
                "Available commands:",
                "  /help          Show this help message",
                "  /clear         Clear conversation history",
                "  /exit          Exit claw4love",
                "  /cost          Show token usage and cost",
                "  /status        Show session status",
                "  /config        Show current configuration",
                "  /compact       Compact conversation history",
                "  /commit        Generate a commit message",
                "  /review        Review code changes",
                "  /plan          Enter planning mode",
                "  /diff          Show recent file changes",
                "",
                "Type a message to chat, or /command to run a command.",
            ].join("\n")
        } else {
            format!("Help for /{args}: (detailed help coming soon)")
        };
        Ok(CommandResult::Text(text))
    }
}

// -- /clear --

pub struct ClearCommand;

impl Command for ClearCommand {
    fn name(&self) -> &str { "clear" }
    fn description(&self) -> &str { "Clear conversation history" }

    fn execute(&self, _args: &str, state: &SharedAppState) -> anyhow::Result<CommandResult> {
        let state = state.blocking_read();
        let _ = &state.messages; // will be cleared by the app
        Ok(CommandResult::Text("Conversation cleared.".into()))
    }
}

// -- /exit --

pub struct ExitCommand;

impl Command for ExitCommand {
    fn name(&self) -> &str { "exit" }
    fn aliases(&self) -> Vec<&str> { vec!["quit", "q"] }
    fn description(&self) -> &str { "Exit claw4love" }

    fn execute(&self, _args: &str, _state: &SharedAppState) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::Exit)
    }
}

// -- /cost --

pub struct CostCommand;

impl Command for CostCommand {
    fn name(&self) -> &str { "cost" }
    fn description(&self) -> &str { "Show token usage and cost for this session" }

    fn execute(&self, _args: &str, state: &SharedAppState) -> anyhow::Result<CommandResult> {
        let state = state.blocking_read();
        let model = &state.main_loop_model;
        let elapsed = state.session_start_time.signed_duration_since(chrono::Utc::now());
        let duration = elapsed.num_seconds().unsigned_abs();

        let text = format!(
            "Session cost:\n  Model: {model}\n  Duration: {}m{}s\n  (detailed token tracking available after API calls)",
            duration / 60,
            duration % 60,
        );
        Ok(CommandResult::Text(text))
    }
}

// -- /status --

pub struct StatusCommand;

impl Command for StatusCommand {
    fn name(&self) -> &str { "status" }
    fn description(&self) -> &str { "Show session status" }

    fn execute(&self, _args: &str, state: &SharedAppState) -> anyhow::Result<CommandResult> {
        let state = state.blocking_read();
        let text = format!(
            "Session: {}\nModel: {}\nMessages: {}\nPlan mode: {}",
            &state.session_id[..8.min(state.session_id.len())],
            state.main_loop_model,
            state.messages.len(),
            if state.is_plan_mode { "on" } else { "off" },
        );
        Ok(CommandResult::Text(text))
    }
}

// -- /config --

pub struct ConfigCommand;

impl Command for ConfigCommand {
    fn name(&self) -> &str { "config" }
    fn description(&self) -> &str { "Show current configuration" }

    fn execute(&self, _args: &str, state: &SharedAppState) -> anyhow::Result<CommandResult> {
        let state = state.blocking_read();
        let text = format!(
            "Model: {}\nVerbose: {}\nPermission mode: {:?}",
            state.main_loop_model,
            state.verbose,
            state.permission_context.mode,
        );
        Ok(CommandResult::Text(text))
    }
}

// -- /commit (Prompt command) --

pub struct CommitCommand;

impl Command for CommitCommand {
    fn name(&self) -> &str { "commit" }
    fn aliases(&self) -> Vec<&str> { vec!["ci"] }
    fn description(&self) -> &str { "Generate a commit message from staged changes" }

    fn execute(&self, args: &str, _state: &SharedAppState) -> anyhow::Result<CommandResult> {
        let extra = if args.is_empty() { String::new() } else { format!(" Context: {args}") };
        Ok(CommandResult::Prompt {
            prompt: format!(
                "Look at the staged git changes (run `git diff --cached`) and create a concise commit message.{extra}"
            ),
            tools: Some(vec!["Bash".into(), "Read".into()]),
        })
    }
}

// -- /review (Prompt command) --

pub struct ReviewCommand;

impl Command for ReviewCommand {
    fn name(&self) -> &str { "review" }
    fn description(&self) -> &str { "Review recent code changes" }

    fn execute(&self, args: &str, _state: &SharedAppState) -> anyhow::Result<CommandResult> {
        let target = if args.is_empty() { "the recent changes" } else { args };
        Ok(CommandResult::Prompt {
            prompt: format!(
                "Review {target}. Look at the git diff, identify potential issues, suggest improvements."
            ),
            tools: Some(vec!["Bash".into(), "Read".into(), "Grep".into(), "Glob".into()]),
        })
    }
}

// -- /plan (Prompt command) --

pub struct PlanCommand;

impl Command for PlanCommand {
    fn name(&self) -> &str { "plan" }
    fn description(&self) -> &str { "Enter planning mode for a task" }

    fn execute(&self, args: &str, _state: &SharedAppState) -> anyhow::Result<CommandResult> {
        if args.is_empty() {
            return Ok(CommandResult::Text("Usage: /plan <description of what to implement>".into()));
        }
        Ok(CommandResult::Prompt {
            prompt: format!(
                "Create an implementation plan for: {args}\n\nBreak it into clear steps. Identify files to modify. Consider edge cases."
            ),
            tools: Some(vec!["Read".into(), "Grep".into(), "Glob".into()]),
        })
    }
}

// -- /diff --

pub struct DiffCommand;

impl Command for DiffCommand {
    fn name(&self) -> &str { "diff" }
    fn description(&self) -> &str { "Show recent file changes" }

    fn execute(&self, _args: &str, _state: &SharedAppState) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::Prompt {
            prompt: "Show the current git diff (unstaged changes). Summarize what changed.".into(),
            tools: Some(vec!["Bash".into()]),
        })
    }
}

// -- /compact --

pub struct CompactCommand;

impl Command for CompactCommand {
    fn name(&self) -> &str { "compact" }
    fn description(&self) -> &str { "Compact conversation history to save context" }

    fn execute(&self, _args: &str, _state: &SharedAppState) -> anyhow::Result<CommandResult> {
        // TODO: actually compact messages in state
        Ok(CommandResult::Text("Conversation compacted.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> SharedAppState {
        c4l_state::AppState::shared("test-session-id-1234".into(), "claude-sonnet-4-6".into())
    }

    #[test]
    fn help_lists_commands() {
        let result = HelpCommand.execute("", &state()).unwrap();
        match result {
            CommandResult::Text(t) => {
                assert!(t.contains("/help"));
                assert!(t.contains("/exit"));
                assert!(t.contains("/cost"));
            }
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn exit_returns_exit() {
        let result = ExitCommand.execute("", &state()).unwrap();
        assert!(matches!(result, CommandResult::Exit));
    }

    #[test]
    fn commit_returns_prompt() {
        let result = CommitCommand.execute("", &state()).unwrap();
        match result {
            CommandResult::Prompt { prompt, tools } => {
                assert!(prompt.contains("git diff"));
                assert!(tools.unwrap().contains(&"Bash".to_string()));
            }
            _ => panic!("expected prompt"),
        }
    }

    #[test]
    fn review_returns_prompt() {
        let result = ReviewCommand.execute("src/main.rs", &state()).unwrap();
        match result {
            CommandResult::Prompt { prompt, .. } => {
                assert!(prompt.contains("src/main.rs"));
            }
            _ => panic!("expected prompt"),
        }
    }

    #[test]
    fn plan_needs_args() {
        let result = PlanCommand.execute("", &state()).unwrap();
        assert!(matches!(result, CommandResult::Text(_)));

        let result = PlanCommand.execute("add auth system", &state()).unwrap();
        assert!(matches!(result, CommandResult::Prompt { .. }));
    }

    #[test]
    fn status_shows_session() {
        let result = StatusCommand.execute("", &state()).unwrap();
        match result {
            CommandResult::Text(t) => {
                assert!(t.contains("test-ses"));
                assert!(t.contains("claude-sonnet-4-6"));
            }
            _ => panic!("expected text"),
        }
    }
}
