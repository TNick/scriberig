//! Provides the shared persistence conformance entry point for backend tests.

use scriberig_store::{FullConformanceStore, run_full_conformance};

/// Runs the complete persistence contract against one backend.
///
/// - `store`: Backend instance under test.
///
/// # Panics
/// Panics when the backend violates the shared persistence contract.
pub fn assert_backend_conformance<S: FullConformanceStore>(store: &S) {
    run_full_conformance(store).expect("shared persistence conformance");
}
