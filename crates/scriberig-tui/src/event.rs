//! Defines terminal events independently from the terminal backend.

use std::io;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode as CrosstermKeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};

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
    /// An observation received from the shared application event boundary.
    Application(scriberig_app::ApplicationEvent),
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
/// Returns the terminal backend error when polling or reading fails.
/// - `timeout`: Argument named `timeout`.
///
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn read_event(timeout: Duration) -> io::Result<TuiEvent> {
    if !event::poll(timeout)? {
        return Ok(TuiEvent::Tick);
    }

    translate_event(event::read())
}

/// Translates one Crossterm event at the terminal boundary.
///
/// - `result`: Terminal event or source read failure.
///
/// Returns the source event error unchanged.
/// - `crate`: Argument named `crate`.
///
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn translate_event(result: io::Result<Event>) -> io::Result<TuiEvent> {
    let event = result?;
    Ok(match event {
        Event::Key(key) if key.kind == KeyEventKind::Release => TuiEvent::Tick,
        Event::Key(key) => TuiEvent::Key(translate_key(key)),
        Event::Paste(text) => TuiEvent::Paste(text),
        Event::Resize(width, height) => TuiEvent::Resize { width, height },
        _ => TuiEvent::Tick,
    })
}

/// Translates a Crossterm key without exposing backend types to the runtime.
/// - `key`: Argument named `key`.
///
/// # Returns
/// The operation result.
fn translate_key(key: KeyEvent) -> Key {
    let mut modifiers = Modifiers {
        control: key.modifiers.contains(KeyModifiers::CONTROL),
        alt: key.modifiers.contains(KeyModifiers::ALT),
        shift: key.modifiers.contains(KeyModifiers::SHIFT),
    };
    let code = match key.code {
        CrosstermKeyCode::Char('\u{3}') => {
            modifiers.control = true;
            KeyCode::Char('c')
        }
        CrosstermKeyCode::Char('?') if !modifiers.control && !modifiers.alt => KeyCode::Help,
        CrosstermKeyCode::Char(character)
            if modifiers.control && character.eq_ignore_ascii_case(&'c') =>
        {
            KeyCode::Char('c')
        }
        CrosstermKeyCode::Char(character) => KeyCode::Char(character),
        CrosstermKeyCode::Esc => KeyCode::Esc,
        CrosstermKeyCode::Enter => KeyCode::Enter,
        CrosstermKeyCode::BackTab => {
            modifiers.shift = true;
            KeyCode::Tab
        }
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
    use crossterm::event::{
        Event, KeyCode as CrosstermKeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    };

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

    /// Verifies key-release events do not duplicate text input.
    #[test]
    fn ignores_key_release_events() {
        let event = translate_event(Ok(Event::Key(KeyEvent::new_with_kind(
            CrosstermKeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ))))
        .expect("event");
        assert_eq!(event, TuiEvent::Tick);
    }

    /// Verifies a raw control character is normalized to the quit chord.
    #[test]
    fn translates_raw_control_c_as_quit_key() {
        let event = translate_event(Ok(Event::Key(KeyEvent::new(
            CrosstermKeyCode::Char('\u{3}'),
            crossterm::event::KeyModifiers::NONE,
        ))))
        .expect("event");
        assert_eq!(
            event,
            TuiEvent::Key(super::Key {
                code: KeyCode::Char('c'),
                modifiers: super::Modifiers {
                    control: true,
                    alt: false,
                    shift: false,
                },
            })
        );
    }

    /// Verifies terminal-native Shift+Tab becomes the reverse-focus chord.
    #[test]
    fn translates_backtab_as_shifted_tab() {
        let event = translate_event(Ok(Event::Key(KeyEvent::new(
            CrosstermKeyCode::BackTab,
            KeyModifiers::NONE,
        ))))
        .expect("event");
        assert_eq!(
            event,
            TuiEvent::Key(super::Key {
                code: KeyCode::Tab,
                modifiers: super::Modifiers {
                    control: false,
                    alt: false,
                    shift: true,
                },
            })
        );
    }
}
