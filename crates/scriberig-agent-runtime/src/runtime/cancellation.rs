//! Creates cancellation state for one agent runtime invocation.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Creates a cancellation flag suitable for one runtime invocation.
///
/// A clear, shared cancellation flag.
#[must_use]
/// # Returns
/// The operation result.
pub fn cancellation_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
