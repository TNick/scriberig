//! Runs the event-driven terminal shell.

use std::io::{self, IsTerminal};
use std::time::Duration;

use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    event, render,
    service::{execute_effect, TuiService},
    state::{reduce, TuiState},
    terminal::TerminalGuard,
    TuiConfig,
};
use scriberig_app::ApplicationContext;

/// Runs the interactive shell until a local quit event is received.
///
/// # Errors
/// Returns an I/O error when terminal setup, drawing, or event reading fails.
pub fn run<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    config: &TuiConfig,
) -> io::Result<()> {
    validate_interactive_terminal(
        config.interactive,
        io::stdin().is_terminal(),
        io::stdout().is_terminal(),
    )?;
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut state = TuiState::default();
    let capability_action =
        execute_effect(service, context, crate::effect::Effect::RequestCapabilities);
    reduce(&mut state, capability_action);
    if let Some(entity) = config.initial_entity {
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        let effects = reduce(
            &mut state,
            crate::action::Action::InspectRequested {
                request_id,
                entity,
                snapshot: config.snapshot,
            },
        );
        for effect in effects {
            let action = execute_effect(service, context, effect);
            reduce(&mut state, action);
        }
    }
    terminal.draw(|frame| render::render(frame, &state))?;
    let mut event_cursor = 0_usize;
    while !state.should_quit {
        let input = event::read_event(Duration::from_millis(250))?;
        let events = service
            .application_events(context)
            .map_err(|error| io::Error::other(error.to_string()))?;
        let start = event_cursor.min(events.len());
        let end = (start + 32).min(events.len());
        for application_event in &events[start..end] {
            for effect in
                state.apply_event(&event::TuiEvent::Application(application_event.clone()))
            {
                let action = execute_effect(service, context, effect);
                reduce(&mut state, action);
            }
        }
        event_cursor = end;
        let redraw = !matches!(input, event::TuiEvent::Tick);
        let effects = state.apply_event(&input);
        for effect in effects {
            let action = execute_effect(service, context, effect);
            reduce(&mut state, action);
        }
        if redraw {
            terminal.draw(|frame| render::render(frame, &state))?;
        }
    }
    Ok(())
}

/// Rejects interactive startup when the caller does not have a terminal.
/// - `interactive`: Argument named `interactive`.
/// - `stdin_is_terminal`: Argument named `stdin_is_terminal`.
/// - `stdout_is_terminal`: Argument named `stdout_is_terminal`.
///
/// # Returns
/// The value produced by the declaration.
///
/// # Errors
/// Returns an error when the operation cannot complete.
fn validate_interactive_terminal(
    interactive: bool,
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
) -> io::Result<()> {
    if interactive && (!stdin_is_terminal || !stdout_is_terminal) {
        return Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "interactive TUI requires a terminal on stdin and stdout",
        ));
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

    /// Verifies non-interactive startup is rejected before terminal takeover.
    #[test]
    fn rejects_noninteractive_terminal_for_interactive_runtime() {
        let error = super::validate_interactive_terminal(true, false, true)
            .expect_err("non-terminal input must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::NotConnected);
    }

    /// Verifies headless composition does not require an attached terminal.
    #[test]
    fn accepts_noninteractive_composition_without_terminal() {
        super::validate_interactive_terminal(false, false, false)
            .expect("headless composition must remain usable");
    }
}
