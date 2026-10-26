//! Implements runtime capability reads for the local application adapter.

use super::ApplicationAdapter;
use crate::service::{ApplicationContext, TuiRuntimeService};
use scriberig_app::{ApplicationError, RuntimeCapability};

impl<S> TuiRuntimeService for ApplicationAdapter<S> {
    /// Returns the capabilities supplied by the composition root.
    ///
    /// - `_context`: Application context for capability access.
    ///
    /// # Returns
    /// The configured runtime capabilities.
    ///
    /// # Errors
    /// This adapter does not fail capability reads.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(self.capabilities.clone())
    }
}
