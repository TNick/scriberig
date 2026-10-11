//! Defines terminal events independently from the terminal backend.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode as CrosstermKeyCode, KeyEvent, KeyModifiers};

/// A terminal event understood by the TUI runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TuiEvent {
    /// A keyboard input event.
    Key(Key),
    /// Text delivered by bracketed paste.
    Paste(String),
    /// A terminal resize notification.
    Resize {
        /// The new terminal width in columns.
        width: u16,
        /// The new terminal height in rows.
        height: u16,
    },
    /// A bounded runtime heartbeat.
    Tick,
    /// A request to stop the runtime.
    Shutdown,
}

/// A backend-independent keyboard event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Key {
    /// The key's logical code.
    pub code: KeyCode,
    /// The modifier keys held with the key.
    pub modifiers: Modifiers,
}

/// Logical keyboard codes used by the TUI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyCode {
    /// A printable character.
    Char(char),
    /// The Escape key.
    Esc,
    /// The Enter key.
    Enter,
    /// The Tab key.
    Tab,
    /// The Backspace key.
    Backspace,
    /// The Up arrow key.
    Up,
    /// The Down arrow key.
    Down,
    /// The Left arrow key.
    Left,
    /// The Right arrow key.
    Right,
    /// The question-mark help key.
    Help,
    /// An unsupported or non-actionable key.
    Other,
}

/// Keyboard modifiers used by the TUI.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    /// Whether Control is held.
    pub control: bool,
    /// Whether Alt is held.
    pub alt: bool,
    /// Whether Shift is held.
    pub shift: bool,
}

/// Reads one translated event when one is available before the timeout.
///
/// # Errors
/// Returns the terminal backend error when polling or reading fails.
pub fn read_event(timeout: Duration) -> io::Result<TuiEvent> {
    if !event::poll(timeout)? {
        return Ok(TuiEvent::Tick);
    }

    translate_event(event::read())
}

/// Translates one Crossterm event at the terminal boundary.
///
/// # Errors
/// Returns the source event error unchanged.
pub(crate) fn translate_event(result: io::Result<Event>) -> io::Result<TuiEvent> {
    let event = result?;
    Ok(match event {
        Event::Key(key) => TuiEvent::Key(translate_key(key)),
        Event::Paste(text) => TuiEvent::Paste(text),
        Event::Resize(width, height) => TuiEvent::Resize { width, height },
        _ => TuiEvent::Tick,
    })
}

/// Translates a Crossterm key without exposing backend types to the runtime.
fn translate_key(key: KeyEvent) -> Key {
    let modifiers = Modifiers {
        control: key.modifiers.contains(KeyModifiers::CONTROL),
        alt: key.modifiers.contains(KeyModifiers::ALT),
        shift: key.modifiers.contains(KeyModifiers::SHIFT),
    };
    let code = match key.code {
        CrosstermKeyCode::Char('?') if !modifiers.control && !modifiers.alt => KeyCode::Help,
        CrosstermKeyCode::Char(character) => KeyCode::Char(character),
        CrosstermKeyCode::Esc => KeyCode::Esc,
        CrosstermKeyCode::Enter => KeyCode::Enter,
        CrosstermKeyCode::Tab => KeyCode::Tab,
        CrosstermKeyCode::Backspace => KeyCode::Backspace,
        CrosstermKeyCode::Up => KeyCode::Up,
        CrosstermKeyCode::Down => KeyCode::Down,
        CrosstermKeyCode::Left => KeyCode::Left,
        CrosstermKeyCode::Right => KeyCode::Right,
        _ => KeyCode::Other,
    };
    Key { code, modifiers }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode as CrosstermKeyCode, KeyEvent};

    use super::{KeyCode, TuiEvent, translate_event};

    /// Verifies backend key events become stable TUI key events.
    #[test]
    fn translates_help_key() {
        let event = translate_event(Ok(Event::Key(KeyEvent::from(CrosstermKeyCode::Char('?')))))
            .expect("event");
        assert_eq!(
            event,
            TuiEvent::Key(super::Key {
                code: KeyCode::Help,
                modifiers: super::Modifiers::default(),
            })
        );
    }

    /// Verifies unsupported backend events are safely ignored.
    #[test]
    fn ignores_unsupported_events() {
        let event = translate_event(Ok(Event::FocusGained)).expect("event");
        assert_eq!(event, TuiEvent::Tick);
    }
}
