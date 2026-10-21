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
    restore: fn() -> io::Result<()>,
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
        let previous_hook = install_panic_restoration_hook(Self::restore_terminal);
        Ok(Self {
            previous_hook: Some(previous_hook),
            restored: false,
            restore: Self::restore_terminal,
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
        let result = restore_once(&mut self.restored, self.restore);
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

/// Runs an interactive operation while guaranteeing terminal cleanup on return.
///
/// - `guard`: Terminal lifecycle owner to restore when the operation exits.
/// - `operation`: Event-loop and terminal work to execute under the guard.
///
/// The operation result after terminal cleanup has run.
pub(crate) fn run_with_terminal_guard<T>(
    guard: TerminalGuard,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    let result = operation();
    drop(guard);
    result
}

/// Runs terminal restoration once, including when the caller requests cleanup twice.
///
/// - `restored`: State tracking whether restoration already ran.
/// - `restore`: Terminal cleanup action to run on the first request.
///
/// # Returns
/// The first cleanup result, or success when cleanup already ran.
/// # Errors
/// Returns an error when the first restoration attempt fails.
fn restore_once(restored: &mut bool, restore: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
    if *restored {
        return Ok(());
    }
    let result = restore();
    *restored = true;
    result
}

/// Installs restoration before delegating panic reporting to the prior hook.
///
/// - `restore`: Terminal restoration action to attempt on panic.
///
/// The previous panic hook, which the owner can restore during teardown.
/// # Returns
/// The previous process panic hook.
fn install_panic_restoration_hook(restore: fn() -> io::Result<()>) -> PanicHook {
    let previous_hook: PanicHook = Arc::from(panic::take_hook());
    let panic_hook = Arc::clone(&previous_hook);
    panic::set_hook(Box::new(move |panic| {
        let _ = restore();
        panic_hook(panic);
    }));
    previous_hook
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
/// # Returns
/// Success when all restoration commands succeed.
/// # Errors
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
#[path = "terminal_tests.rs"]
mod tests;
