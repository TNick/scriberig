//! Runs bounded validation child processes and captures their output.

use std::io::Read;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Represents cancellation or failure while running candidate validation.
pub(super) enum ProcessFailure {
    /// Validation was cancelled by its owning operation.
    Cancelled,
    /// Validation could not complete and has a diagnostic.
    Failed(String),
}

/// Runs a validation command with a deadline and bounded captured output.
///
/// # Parameters
/// - `command`: Configured validation process.
/// - `timeout`: Maximum process duration.
/// - `cancelled`: Shared cancellation flag for the owning operation.
/// - `super`: Parent candidate validation module.
///
/// # Returns
/// Process output with each stream limited to 64 KiB.
///
/// # Errors
/// Returns a structured message for startup, timeout, or output-reader failure.
pub(super) fn run_bounded(
    mut command: Command,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<Output, ProcessFailure> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| ProcessFailure::Failed(format!("start failure: {error}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ProcessFailure::Failed("stdout pipe unavailable".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ProcessFailure::Failed("stderr pipe unavailable".to_owned()))?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr));
    let status = wait_bounded(&mut child, timeout, cancelled)?;
    let stdout = stdout_reader
        .join()
        .map_err(|_| ProcessFailure::Failed("stdout reader failed".to_owned()))?
        .map_err(ProcessFailure::Failed)?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| ProcessFailure::Failed("stderr reader failed".to_owned()))?
        .map_err(ProcessFailure::Failed)?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Waits for a child process and terminates it when its deadline expires.
///
/// # Parameters
/// - `child`: Running validation process.
/// - `timeout`: Maximum process duration.
/// - `cancelled`: Shared cancellation flag for the owning operation.
///
/// # Returns
/// The completed process status.
///
/// # Errors
/// Returns a timeout or process polling failure.
fn wait_bounded(
    child: &mut Child,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<ExitStatus, ProcessFailure> {
    let deadline = Instant::now() + timeout;
    loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProcessFailure::Cancelled);
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| ProcessFailure::Failed(error.to_string()))?
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProcessFailure::Failed(format!(
                "timeout after {} seconds",
                timeout.as_secs()
            )));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Drains a process stream while retaining only a bounded prefix.
///
/// # Parameters
/// - `reader`: Child process output stream.
///
/// # Returns
/// At most 64 KiB of output.
///
/// # Errors
/// Returns a stream read failure.
fn read_bounded(mut reader: impl Read) -> Result<Vec<u8>, String> {
    let mut retained = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(retained);
        }
        let remaining = 65_536_usize.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..count.min(remaining)]);
    }
}
