//! Verifies bounded validation process control.

use super::run_bounded;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Verifies that validation processes are terminated at their deadline.
#[test]
fn validation_process_timeout_is_reported() {
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 2"]);
    let error = run_bounded(command, Duration::from_millis(20), &AtomicBool::new(false))
        .expect_err("slow process must time out");
    assert!(
        matches!(error, super::ProcessFailure::Failed(error) if error.starts_with("timeout after"))
    );
}

/// Verifies that cancellation terminates a running validation process.
#[test]
fn validation_process_cancellation_is_reported() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    let setter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        signal.store(true, Ordering::Release);
    });
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 5"]);
    let error = run_bounded(command, Duration::from_secs(2), &cancelled)
        .expect_err("cancelled process must stop");
    setter.join().expect("cancellation setter must finish");
    assert!(matches!(error, super::ProcessFailure::Cancelled));
}
