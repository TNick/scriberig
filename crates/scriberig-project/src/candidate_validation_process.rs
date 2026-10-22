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

/// Retains the diagnostic tail within a character limit.
///
/// - `value`: Complete process output.
/// - `maximum`: Maximum number of Unicode scalar values to retain.
///
/// The most recent bounded portion of the output.
/// # Returns
/// The output suffix bounded by `maximum` Unicode scalar values.
pub(super) fn bounded_tail(value: &str, maximum: usize) -> String {
    let mut tail = value.chars().rev().take(maximum).collect::<Vec<_>>();
    tail.reverse();
    tail.into_iter().collect()
}

/// Runs a validation command with a deadline and bounded captured output.
///
/// - `command`: Configured validation process.
/// - `timeout`: Maximum process duration.
/// - `cancelled`: Shared cancellation flag for the owning operation.
/// - `super`: Parent candidate validation module.
///
/// Process output with each stream limited to its final 64 KiB.
///
/// Returns a structured message for startup, timeout, or output-reader failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `child`: Running validation process.
/// - `timeout`: Maximum process duration.
/// - `cancelled`: Shared cancellation flag for the owning operation.
///
/// The completed process status.
///
/// Returns a timeout or process polling failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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

/// Drains a process stream while retaining its bounded suffix.
///
/// - `reader`: Child process output stream.
///
/// The final 64 KiB of output.
///
/// Returns a stream read failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
        retained.extend_from_slice(&buffer[..count]);
        if retained.len() > 65_536 {
            let excess = retained.len() - 65_536;
            retained.drain(..excess);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{bounded_tail, read_bounded};

    /// Verifies bounded validation diagnostics retain their final error details.
    #[test]
    fn bounded_validation_diagnostics_retain_the_tail() {
        assert_eq!(bounded_tail("0123456789", 4), "6789");
        assert_eq!(bounded_tail("ééabc", 3), "abc");
    }

    /// Verifies process capture preserves errors emitted after large build logs.
    #[test]
    fn bounded_process_output_retains_the_final_bytes() {
        let mut bytes = vec![b'a'; 65_540];
        bytes[65_536..].copy_from_slice(b"tail");

        let output = read_bounded(Cursor::new(bytes)).expect("read process output");

        assert_eq!(output.len(), 65_536);
        assert_eq!(&output[output.len() - 4..], b"tail");
    }
}
