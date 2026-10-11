//! Runs the event-driven terminal shell.

use std::io;
use std::time::Duration;

use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    TuiConfig, event, render,
    service::{TuiService, execute_effect},
    state::{TuiState, reduce},
    terminal::TerminalGuard,
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
    while !state.should_quit {
        let input = event::read_event(Duration::from_millis(250))?;
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
