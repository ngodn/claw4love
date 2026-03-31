//! Command trait and result types.

use c4l_state::SharedAppState;

/// Result of executing a slash command.
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Text output displayed to the user.
    Text(String),
    /// Formatted prompt sent to the LLM with optional tool restrictions.
    Prompt {
        prompt: String,
        tools: Option<Vec<String>>,
    },
    /// Side effect only, no output.
    None,
    /// Exit the application.
    Exit,
}

/// Slash command trait.
///
/// Maps from: TypeScript Command types (PromptCommand, LocalCommand, LocalJSXCommand)
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }
    fn description(&self) -> &str;

    /// Execute the command with the given arguments.
    fn execute(
        &self,
        args: &str,
        state: &SharedAppState,
    ) -> anyhow::Result<CommandResult>;
}
