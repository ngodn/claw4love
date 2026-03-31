//! Command registry: holds all registered commands and provides dispatch.

use crate::traits::{Command, CommandResult};
use c4l_state::SharedAppState;

/// Registry of slash commands.
pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn register(&mut self, cmd: impl Command + 'static) {
        self.commands.push(Box::new(cmd));
    }

    /// Find a command by name or alias (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&dyn Command> {
        let name_lower = name.to_lowercase();
        self.commands
            .iter()
            .find(|c| {
                c.name().eq_ignore_ascii_case(&name_lower)
                    || c.aliases().iter().any(|a| a.eq_ignore_ascii_case(&name_lower))
            })
            .map(|c| c.as_ref())
    }

    pub fn all(&self) -> &[Box<dyn Command>] {
        &self.commands
    }

    /// Parse and dispatch a slash command string (e.g., "/help commit").
    /// Returns None if the input doesn't start with '/'.
    pub fn dispatch(
        &self,
        input: &str,
        state: &SharedAppState,
    ) -> Option<anyhow::Result<CommandResult>> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let without_slash = &input[1..];
        let (cmd_name, args) = match without_slash.split_once(char::is_whitespace) {
            Some((name, rest)) => (name, rest.trim()),
            None => (without_slash, ""),
        };

        let command = self.get(cmd_name)?;
        Some(command.execute(args, state))
    }

    /// Register all built-in commands.
    pub fn register_defaults(&mut self) {
        self.register(crate::builtins::HelpCommand);
        self.register(crate::builtins::ClearCommand);
        self.register(crate::builtins::ExitCommand);
        self.register(crate::builtins::CostCommand);
        self.register(crate::builtins::StatusCommand);
        self.register(crate::builtins::ConfigCommand);
        self.register(crate::builtins::CommitCommand);
        self.register(crate::builtins::ReviewCommand);
        self.register(crate::builtins::PlanCommand);
        self.register(crate::builtins::DiffCommand);
        self.register(crate::builtins::CompactCommand);
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register_defaults();
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_commands() {
        let reg = CommandRegistry::default();
        assert!(reg.get("help").is_some());
        assert!(reg.get("clear").is_some());
        assert!(reg.get("exit").is_some());
        assert!(reg.get("cost").is_some());
    }

    #[tokio::test]
    async fn dispatch_slash_command() {
        let reg = CommandRegistry::default();
        let state = c4l_state::AppState::shared("test".into(), "model".into());

        let result = reg.dispatch("/help", &state);
        assert!(result.is_some());
        let output = result.unwrap().unwrap();
        match output {
            CommandResult::Text(t) => assert!(t.contains("/help")),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn dispatch_non_command_returns_none() {
        let reg = CommandRegistry::default();
        let state = c4l_state::AppState::shared("test".into(), "model".into());

        assert!(reg.dispatch("hello world", &state).is_none());
    }

    #[test]
    fn dispatch_unknown_command_returns_none() {
        let reg = CommandRegistry::default();
        let state = c4l_state::AppState::shared("test".into(), "model".into());

        assert!(reg.dispatch("/nonexistent", &state).is_none());
    }

    #[tokio::test]
    async fn dispatch_with_args() {
        let reg = CommandRegistry::default();
        let state = c4l_state::AppState::shared("test".into(), "model".into());

        let result = reg.dispatch("/help commit", &state);
        assert!(result.is_some());
    }
}
