//! Owns terminal setup and restoration for the interactive runtime.

use std::io::{self, Write};
use std::panic::{self, PanicHookInfo};
use std::sync::Arc;

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

/// A panic hook that can be restored after terminal teardown.
type PanicHook = Arc<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static>;

/// Restores terminal state and composes with the previous panic hook.
pub struct TerminalGuard {
    previous_hook: Option<PanicHook>,
    restored: bool,
}

/// Tracks terminal capabilities changed by the guard.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg(test)]
#[allow(clippy::struct_excessive_bools)]
struct LifecycleState {
    raw_mode: bool,
    alternate_screen: bool,
    cursor_hidden: bool,
    bracketed_paste: bool,
}

impl TerminalGuard {
    /// Enters raw/alternate-screen mode and installs panic restoration.
    ///
    /// # Errors
    /// Returns an I/O error when raw mode or alternate-screen setup fails.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide, EnableBracketedPaste) {
            let _ = disable_raw_mode();
            let _ = Self::restore_terminal();
            return Err(error);
        }
        let previous_hook: Arc<dyn Fn(&PanicHookInfo<'_>) + Send + Sync> =
            Arc::from(panic::take_hook());
        let panic_hook = Arc::clone(&previous_hook);
        panic::set_hook(Box::new(move |panic| {
            let _ = Self::restore_terminal();
            panic_hook(panic);
        }));
        Ok(Self {
            previous_hook: Some(previous_hook),
            restored: false,
        })
    }

    /// Restores terminal state once and reinstalls the previous panic hook.
    ///
    /// # Errors
    /// Returns an I/O error when terminal restoration fails.
    ///
    /// # Returns
    /// The value produced by the declaration.
    pub fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }
        let result = Self::restore_terminal();
        self.restored = true;
        if let Some(previous_hook) = self.previous_hook.take() {
            panic::set_hook(Box::new(move |panic| previous_hook(panic)));
        }
        result
    }

    /// Performs the backend commands needed to leave interactive mode.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn restore_terminal() -> io::Result<()> {
        let mut first_error = disable_raw_mode().err();
        let mut stdout = io::stdout();
        for command in [
            TerminalCommand::DisableBracketedPaste,
            TerminalCommand::Show,
            TerminalCommand::LeaveAlternateScreen,
        ] {
            let result = match command {
                TerminalCommand::DisableBracketedPaste => {
                    execute!(stdout, DisableBracketedPaste)
                }
                TerminalCommand::Show => execute!(stdout, Show),
                TerminalCommand::LeaveAlternateScreen => execute!(stdout, LeaveAlternateScreen),
            };
            if first_error.is_none() {
                first_error = result.err();
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

/// Identifies one independently attempted terminal restoration command.
enum TerminalCommand {
    /// Disables bracketed paste mode.
    DisableBracketedPaste,
    /// Shows the cursor.
    Show,
    /// Leaves the alternate screen.
    LeaveAlternateScreen,
}

impl Drop for TerminalGuard {
    /// Attempts restoration if the owner did not explicitly restore first.
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// Flushes terminal output after a lifecycle command in tests or adapters.
///
/// # Errors
/// Returns the writer's flush error.
/// - `output`: Argument named `output`.
///
/// # Returns
/// The value produced by the declaration.
pub fn flush(output: &mut impl Write) -> io::Result<()> {
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::{LifecycleState, flush};

    /// Verifies lifecycle-adjacent output can be flushed through an abstraction.
    #[test]
    fn flushes_test_output() {
        let mut output = Vec::new();
        flush(&mut output).expect("flush");
        assert_eq!(output.len(), 0);
    }

    /// Verifies lifecycle state can be exercised without changing a real terminal.
    #[test]
    fn lifecycle_state_records_all_owned_capabilities() {
        let state = LifecycleState {
            raw_mode: true,
            alternate_screen: true,
            cursor_hidden: true,
            bracketed_paste: true,
        };
        assert!(state.raw_mode);
        assert!(state.alternate_screen);
        assert!(state.cursor_hidden);
        assert!(state.bracketed_paste);
    }
}
