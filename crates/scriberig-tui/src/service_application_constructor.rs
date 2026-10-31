//! Provides construction and borrowing methods for the application adapter.

use super::ApplicationAdapter;
use scriberig_app::RuntimeCapability;

impl<S> ApplicationAdapter<S> {
    /// Creates a TUI adapter around a public application service.
    ///
    /// - `service`: Public application service.
    /// - `capabilities`: Runtime capabilities exposed by the service.
    ///
    /// # Returns
    /// A configured TUI adapter.
    #[must_use]
    pub fn new(service: S, capabilities: Vec<RuntimeCapability>) -> Self {
        Self {
            service,
            capabilities,
        }
    }

    /// Borrows the wrapped application service.
    ///
    /// # Returns
    /// The original service instance.
    #[must_use]
    pub const fn service(&self) -> &S {
        &self.service
    }
}
