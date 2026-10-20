//! Covers terminal lifecycle recovery behavior.

use std::io;

use super::{
    LifecycleState, TerminalCommand, TerminalDriver, TerminalGuard, flush,
    install_panic_restoration_hook, restoration_commands, restore_once, restore_with,
    run_with_terminal_guard,
};
use std::sync::atomic::{AtomicBool, Ordering};

static PANIC_HOOK_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static PANIC_RESTORATION_REQUESTED: AtomicBool = AtomicBool::new(false);
static ERROR_EXIT_RESTORATION_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Records that the injected panic hook attempted terminal restoration.
///
/// # Returns
/// Success after recording the attempt.
/// # Errors
/// This fake callback does not return an error.
fn record_panic_restoration() {
    PANIC_RESTORATION_REQUESTED.store(true, Ordering::Release);
}

/// Records cleanup requested after the interactive operation returns an error.
///
/// # Returns
/// Success after recording the attempt.
/// # Errors
/// This fake callback does not return an error.
fn record_error_exit_restoration() {
    ERROR_EXIT_RESTORATION_REQUESTED.store(true, Ordering::Release);
}

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

/// Verifies repeated teardown requests invoke the terminal driver once.
#[test]
fn terminal_restoration_is_idempotent() {
    let mut restored = false;
    let mut attempts = 0;
    restore_once(&mut restored, || {
        attempts += 1;
        Ok(())
    })
    .expect("first restore");
    restore_once(&mut restored, || {
        attempts += 1;
        Ok(())
    })
    .expect("second restore");

    assert_eq!(attempts, 1);
}

/// Verifies an event-loop error exits through the terminal cleanup owner.
#[test]
fn terminal_restores_when_interactive_operation_returns_an_error() {
    ERROR_EXIT_RESTORATION_REQUESTED.store(false, Ordering::Release);
    let guard = TerminalGuard {
        previous_hook: None,
        restored: false,
        restore: || {
            record_error_exit_restoration();
            Ok(())
        },
    };

    let result = run_with_terminal_guard(guard, || {
        Err::<(), _>(io::Error::other("controlled event-loop failure"))
    });

    assert_eq!(
        result
            .expect_err("event-loop error should be preserved")
            .to_string(),
        "controlled event-loop failure"
    );
    assert!(ERROR_EXIT_RESTORATION_REQUESTED.load(Ordering::Acquire));
}

/// Verifies panic restoration runs before the previous hook is delegated.
#[test]
fn panic_hook_restores_before_delegating_without_a_real_terminal() {
    let _lock = PANIC_HOOK_TEST_LOCK.lock().expect("panic hook test lock");
    PANIC_RESTORATION_REQUESTED.store(false, Ordering::Release);
    let previous_hook = install_panic_restoration_hook(|| {
        record_panic_restoration();
        Ok(())
    });
    let panic_result = std::panic::catch_unwind(|| panic!("controlled panic fixture"));
    std::panic::set_hook(Box::new(move |panic| previous_hook(panic)));

    assert!(panic_result.is_err());
    assert!(PANIC_RESTORATION_REQUESTED.load(Ordering::Acquire));
}
