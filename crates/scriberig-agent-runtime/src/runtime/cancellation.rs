//! Creates cancellation state for one agent runtime invocation.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Creates a cancellation flag suitable for one runtime invocation.
///
/// # Returns
/// A clear, shared cancellation flag.
#[must_use]
pub fn cancellation_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
