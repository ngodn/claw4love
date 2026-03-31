//! Text input state with editing, history, and key handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// What to do after a key press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    /// Keep editing, nothing special.
    Continue,
    /// User pressed Enter: submit the current text.
    Submit(String),
    /// User pressed Escape.
    Cancel,
    /// User pressed Ctrl+C.
    Interrupt,
    /// Scroll request (Page Up / Page Down).
    ScrollUp,
    ScrollDown,
}

/// Multi-line text input with cursor and history.
pub struct InputState {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            history: Vec::new(),
            history_index: None,
        }
    }

    /// Get the full input text (all lines joined).
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.is_empty())
    }

    /// Clear input and reset cursor.
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.history_index = None;
    }

    /// Current line being edited.
    fn current_line(&self) -> &str {
        &self.lines[self.cursor_row]
    }

    /// Handle a key event and return the resulting action.
    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction {
        match key.code {
            // Ctrl+C: interrupt
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                InputAction::Interrupt
            }

            // Enter: submit (Shift+Enter or Alt+Enter for newline)
            KeyCode::Enter
                if !key.modifiers.contains(KeyModifiers::SHIFT)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                let text = self.content();
                if !text.trim().is_empty() {
                    self.history.push(text.clone());
                    self.clear();
                    InputAction::Submit(text)
                } else {
                    InputAction::Continue
                }
            }

            // Shift+Enter or Alt+Enter: insert newline
            KeyCode::Enter => {
                let rest = self.lines[self.cursor_row][self.cursor_col..].to_string();
                self.lines[self.cursor_row].truncate(self.cursor_col);
                self.cursor_row += 1;
                self.lines.insert(self.cursor_row, rest);
                self.cursor_col = 0;
                InputAction::Continue
            }

            // Escape
            KeyCode::Esc => InputAction::Cancel,

            // Backspace
            KeyCode::Backspace => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    self.lines[self.cursor_row].remove(self.cursor_col);
                } else if self.cursor_row > 0 {
                    // Merge with previous line
                    let current = self.lines.remove(self.cursor_row);
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                    self.lines[self.cursor_row].push_str(&current);
                }
                InputAction::Continue
            }

            // Delete
            KeyCode::Delete => {
                if self.cursor_col < self.current_line().len() {
                    self.lines[self.cursor_row].remove(self.cursor_col);
                } else if self.cursor_row + 1 < self.lines.len() {
                    let next = self.lines.remove(self.cursor_row + 1);
                    self.lines[self.cursor_row].push_str(&next);
                }
                InputAction::Continue
            }

            // Arrow keys
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                }
                InputAction::Continue
            }
            KeyCode::Right => {
                if self.cursor_col < self.current_line().len() {
                    self.cursor_col += 1;
                } else if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
                InputAction::Continue
            }
            KeyCode::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                } else {
                    // History navigation
                    self.navigate_history_back();
                }
                InputAction::Continue
            }
            KeyCode::Down => {
                if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                } else {
                    self.navigate_history_forward();
                }
                InputAction::Continue
            }

            // Home / End
            KeyCode::Home => {
                self.cursor_col = 0;
                InputAction::Continue
            }
            KeyCode::End => {
                self.cursor_col = self.current_line().len();
                InputAction::Continue
            }

            // Page Up / Down
            KeyCode::PageUp => InputAction::ScrollUp,
            KeyCode::PageDown => InputAction::ScrollDown,

            // Character input
            KeyCode::Char(c) => {
                self.lines[self.cursor_row].insert(self.cursor_col, c);
                self.cursor_col += 1;
                self.history_index = None;
                InputAction::Continue
            }

            // Tab: insert spaces
            KeyCode::Tab => {
                self.lines[self.cursor_row].insert_str(self.cursor_col, "  ");
                self.cursor_col += 2;
                InputAction::Continue
            }

            _ => InputAction::Continue,
        }
    }

    fn navigate_history_back(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            Some(i) if i > 0 => i - 1,
            Some(_) => return,
            None => self.history.len() - 1,
        };
        self.history_index = Some(idx);
        let entry = self.history[idx].clone();
        self.lines = entry.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    fn navigate_history_forward(&mut self) {
        match self.history_index {
            Some(i) if i + 1 < self.history.len() => {
                self.history_index = Some(i + 1);
                let entry = self.history[i + 1].clone();
                self.lines = entry.lines().map(String::from).collect();
                if self.lines.is_empty() {
                    self.lines.push(String::new());
                }
                self.cursor_row = self.lines.len() - 1;
                self.cursor_col = self.lines[self.cursor_row].len();
            }
            Some(_) => {
                self.history_index = None;
                self.clear();
            }
            None => {}
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn type_characters() {
        let mut input = InputState::new();
        input.handle_key(key(KeyCode::Char('h')));
        input.handle_key(key(KeyCode::Char('i')));
        assert_eq!(input.content(), "hi");
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn submit_on_enter() {
        let mut input = InputState::new();
        input.handle_key(key(KeyCode::Char('h')));
        input.handle_key(key(KeyCode::Char('i')));
        let action = input.handle_key(key(KeyCode::Enter));
        assert_eq!(action, InputAction::Submit("hi".into()));
        assert!(input.is_empty());
    }

    #[test]
    fn empty_enter_does_nothing() {
        let mut input = InputState::new();
        let action = input.handle_key(key(KeyCode::Enter));
        assert_eq!(action, InputAction::Continue);
    }

    #[test]
    fn backspace_deletes() {
        let mut input = InputState::new();
        input.handle_key(key(KeyCode::Char('a')));
        input.handle_key(key(KeyCode::Char('b')));
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.content(), "a");
    }

    #[test]
    fn ctrl_c_interrupts() {
        let mut input = InputState::new();
        let action = input.handle_key(key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(action, InputAction::Interrupt);
    }

    #[test]
    fn history_navigation() {
        let mut input = InputState::new();
        // Submit two entries
        input.handle_key(key(KeyCode::Char('a')));
        input.handle_key(key(KeyCode::Enter));
        input.handle_key(key(KeyCode::Char('b')));
        input.handle_key(key(KeyCode::Enter));

        assert_eq!(input.history.len(), 2);

        // Up arrow to go back
        input.handle_key(key(KeyCode::Up));
        assert_eq!(input.content(), "b");
        input.handle_key(key(KeyCode::Up));
        assert_eq!(input.content(), "a");

        // Down arrow to go forward
        input.handle_key(key(KeyCode::Down));
        assert_eq!(input.content(), "b");
    }

    #[test]
    fn newline_with_shift_enter() {
        let mut input = InputState::new();
        input.handle_key(key(KeyCode::Char('a')));
        input.handle_key(key_mod(KeyCode::Enter, KeyModifiers::SHIFT));
        input.handle_key(key(KeyCode::Char('b')));
        assert_eq!(input.content(), "a\nb");
        assert_eq!(input.lines.len(), 2);
    }
}
