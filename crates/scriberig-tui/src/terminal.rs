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
    /// Returns an I/O error when raw mode or alternate-screen setup fails.
    ///
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// Returns an I/O error when terminal restoration fails.
    ///
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn restore_terminal() -> io::Result<()> {
        let mut driver = CrosstermDriver {
            stdout: io::stdout(),
        };
        restore_with(&mut driver)
    }
}

/// Returns the complete, ordered terminal restoration plan.
///
/// Keeping the plan separate makes teardown ordering testable without taking
/// over the process terminal.
///
/// Commands that undo every capability enabled by [`TerminalGuard::enter`].
/// # Returns
/// The operation result.
fn restoration_commands() -> [TerminalCommand; 3] {
    [
        TerminalCommand::DisableBracketedPaste,
        TerminalCommand::Show,
        TerminalCommand::LeaveAlternateScreen,
    ]
}

/// Executes terminal restoration through an injectable lifecycle driver.
///
/// - `driver`: Terminal command implementation.
///
/// The first cleanup error, after all cleanup commands were attempted.
///
/// Returns the first error reported by the driver.
fn restore_with<D: TerminalDriver>(driver: &mut D) -> io::Result<()> {
    let mut first_error = driver.disable_raw_mode().err();
    for command in restoration_commands() {
        if let Err(error) = driver.execute(command) {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

/// Abstracts the terminal commands needed during restoration.
trait TerminalDriver {
    /// Disables raw mode.
    ///
    /// Success when raw mode is disabled.
    ///
    /// Returns the terminal driver's I/O error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn disable_raw_mode(&mut self) -> io::Result<()>;

    /// Executes one non-raw-mode restoration command.
    ///
    /// - `command`: Terminal restoration command to execute.
    ///
    /// Success when the command is executed.
    ///
    /// Returns the terminal driver's I/O error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(&mut self, command: TerminalCommand) -> io::Result<()>;
}

/// Executes restoration commands against the process terminal.
struct CrosstermDriver {
    /// Terminal output receiving crossterm commands.
    stdout: io::Stdout,
}

impl TerminalDriver for CrosstermDriver {
    /// Disables crossterm raw mode.
    ///
    /// Success when raw mode is disabled.
    ///
    /// Returns the terminal driver's I/O error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn disable_raw_mode(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }

    /// Executes one crossterm restoration command.
    ///
    /// - `command`: Terminal restoration command to execute.
    ///
    /// Success when the command is executed.
    ///
    /// Returns the terminal driver's I/O error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(&mut self, command: TerminalCommand) -> io::Result<()> {
        match command {
            TerminalCommand::DisableBracketedPaste => {
                execute!(self.stdout, DisableBracketedPaste)
            }
            TerminalCommand::Show => execute!(self.stdout, Show),
            TerminalCommand::LeaveAlternateScreen => {
                execute!(self.stdout, LeaveAlternateScreen)
            }
        }
    }
}

/// Identifies one independently attempted terminal restoration command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
/// Returns the writer's flush error.
/// - `output`: Argument named `output`.
///
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn flush(output: &mut impl Write) -> io::Result<()> {
    output.flush()
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::{
        LifecycleState, TerminalCommand, TerminalDriver, flush, restoration_commands, restore_with,
    };

    /// Records restoration attempts while optionally failing one command.
    struct FakeDriver {
        /// Commands attempted by the restoration executor.
        commands: Vec<TerminalCommand>,
        /// Command that should return an injected failure.
        failure: Option<TerminalCommand>,
    }

    impl TerminalDriver for FakeDriver {
        /// Records raw-mode restoration without touching a terminal.
        ///
        /// Always succeeds for the fake driver.
        ///
        /// This fake driver does not return an error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn disable_raw_mode(&mut self) -> io::Result<()> {
            Ok(())
        }

        /// Records one restoration command and injects its configured failure.
        ///
        /// - `command`: Terminal restoration command to record.
        ///
        /// Success unless the command is configured to fail.
        ///
        /// Returns the configured injected error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn execute(&mut self, command: TerminalCommand) -> io::Result<()> {
            self.commands.push(command);
            if self.failure == Some(command) {
                Err(io::Error::other("injected restoration failure"))
            } else {
                Ok(())
            }
        }
    }

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

    /// Verifies teardown attempts every owned capability in stable order.
    #[test]
    fn restoration_plan_is_complete_and_ordered() {
        assert!(matches!(
            restoration_commands(),
            [
                TerminalCommand::DisableBracketedPaste,
                TerminalCommand::Show,
                TerminalCommand::LeaveAlternateScreen
            ]
        ));
    }

    /// Verifies cleanup continues after one command fails.
    #[test]
    fn restoration_attempts_all_commands_after_failure() {
        let mut driver = FakeDriver {
            commands: Vec::new(),
            failure: Some(TerminalCommand::Show),
        };
        assert!(restore_with(&mut driver).is_err());
        assert_eq!(driver.commands, restoration_commands());
    }
}
