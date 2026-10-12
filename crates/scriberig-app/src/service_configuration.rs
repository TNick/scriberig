//! Provides optional application-service configuration boundaries.

use std::sync::Arc;

use crate::ApplicationService;

impl<B> ApplicationService<B> {
    /// Installs a durable provider-neutral session repository.
    ///
    /// # Parameters
    /// - `self`: Service being configured.
    /// - `store`: Session repository used by local session operations.
    ///
    /// # Returns
    /// The service configured to persist session graphs.
    #[must_use]
    pub fn with_session_store(mut self, store: Arc<dyn scriberig_session::SessionStore>) -> Self {
        self.session_store = Some(store);
        self
    }
}
