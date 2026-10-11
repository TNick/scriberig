//! Stores presentation-only state for the initial terminal shell.

use crate::event::{KeyCode, TuiEvent};

/// The visible shell pane receiving keyboard focus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    /// The semantic explorer pane.
    Explorer,
    /// The primary content pane.
    Primary,
    /// The inspector pane.
    Inspector,
}

/// Runtime and presentation state for the initial TUI shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiState {
    /// Whether the event loop should terminate.
    pub should_quit: bool,
    /// The most recently observed terminal dimensions.
    pub terminal_size: (u16, u16),
    /// The pane that currently owns local keyboard focus.
    pub focus: Focus,
    /// A short status message rendered in the shell.
    pub shell_message: String,
}

impl Default for TuiState {
    /// Creates an empty, deterministic shell state.
    fn default() -> Self {
        Self {
            should_quit: false,
            terminal_size: (80, 24),
            focus: Focus::Primary,
            shell_message: "No semantic service is attached in this runtime.".to_owned(),
        }
    }
}

impl TuiState {
    /// Applies one local terminal event to presentation state.
    pub fn apply_event(&mut self, event: &TuiEvent) {
        match event {
            TuiEvent::Key(key) => self.apply_key(&key.code),
            TuiEvent::Resize { width, height } => {
                self.terminal_size = (*width, *height);
            }
            TuiEvent::Shutdown => self.should_quit = true,
            TuiEvent::Paste(_) | TuiEvent::Tick => {}
        }
    }

    /// Applies one backend-independent key action.
    fn apply_key(&mut self, key: &KeyCode) {
        if matches!(key, KeyCode::Char('q') | KeyCode::Esc) {
            self.should_quit = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TuiState;
    use crate::event::{Key, KeyCode, Modifiers, TuiEvent};

    /// Verifies resize events update only terminal presentation state.
    #[test]
    fn resize_updates_state() {
        let mut state = TuiState::default();
        state.apply_event(&TuiEvent::Resize {
            width: 72,
            height: 24,
        });
        assert_eq!(state.terminal_size, (72, 24));
        assert!(!state.should_quit);
    }

    /// Verifies quit input stops the runtime.
    #[test]
    fn quit_key_sets_quit_flag() {
        let mut state = TuiState::default();
        state.apply_event(&TuiEvent::Key(Key {
            code: KeyCode::Char('q'),
            modifiers: Modifiers::default(),
        }));
        assert!(state.should_quit);
    }
}
