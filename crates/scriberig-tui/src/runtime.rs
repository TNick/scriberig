//! Runs the event-driven terminal shell.

use std::io;
use std::time::Duration;

use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{event, render, state::TuiState, terminal::TerminalGuard};

/// Runs the interactive shell until a local quit event is received.
///
/// # Errors
/// Returns an I/O error when terminal setup, drawing, or event reading fails.
pub fn run() -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut state = TuiState::default();
    terminal.draw(|frame| render::render(frame, &state))?;
    while !state.should_quit {
        let input = event::read_event(Duration::from_millis(250))?;
        let redraw = !matches!(input, event::TuiEvent::Tick);
        state.apply_event(&input);
        if redraw {
            terminal.draw(|frame| render::render(frame, &state))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        event::{Key, KeyCode, Modifiers, TuiEvent},
        state::TuiState,
    };

    /// Verifies a runtime quit event is handled without a terminal backend.
    #[test]
    fn quit_event_terminates_state_loop() {
        let mut state = TuiState::default();
        state.apply_event(&TuiEvent::Key(Key {
            code: KeyCode::Char('q'),
            modifiers: Modifiers::default(),
        }));
        assert!(state.should_quit);
    }
}
