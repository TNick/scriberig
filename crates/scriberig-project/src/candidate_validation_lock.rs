//! Serializes reuse of exact candidate validation workspaces.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Describes cancellation or failure while acquiring the workspace lock.
pub(super) enum ValidationLockError {
    /// The owning validation operation was cancelled.
    Cancelled,
    /// Lock acquisition failed or exceeded its bounded wait.
    Failed(String),
}

/// Acquires the process-wide candidate workspace lock while observing cancellation.
///
/// - `cancelled`: Owning operation's cancellation state.
/// - `super`: Parent candidate validation module.
///
/// A guard protecting deterministic candidate workspace reuse.
///
/// Returns cancellation or a bounded lock timeout.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn acquire(
    cancelled: &AtomicBool,
) -> Result<std::sync::MutexGuard<'static, ()>, ValidationLockError> {
    static VALIDATION_LOCK: Mutex<()> = Mutex::new(());
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(ValidationLockError::Cancelled);
        }
        match VALIDATION_LOCK.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(ValidationLockError::Failed(
                    "candidate validation workspace lock poisoned".to_owned(),
                ));
            }
            Err(std::sync::TryLockError::WouldBlock) if Instant::now() >= deadline => {
                return Err(ValidationLockError::Failed(
                    "candidate validation workspace lock timeout".to_owned(),
                ));
            }
            Err(std::sync::TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(10)),
        }
    }
}
