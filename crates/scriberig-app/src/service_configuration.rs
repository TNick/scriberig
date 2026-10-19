//! Provides optional application-service configuration boundaries.

use std::sync::Arc;

use crate::{ApplicationService, ModelLibrary};

impl<B> ApplicationService<B> {
    /// Installs the shared user presentation-preference store.
    ///
    /// - `self`: Service being configured.
    /// - `store`: User-scoped preference persistence boundary.
    ///
    /// The service configured with the supplied preference store.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_agent_presentation_preference_store(
        mut self,
        store: Arc<dyn crate::AgentPresentationPreferenceStore>,
    ) -> Self {
        self.presentation_preferences = store;
        self
    }

    /// Installs a durable provider-neutral session repository.
    ///
    /// - `self`: Service being configured.
    /// - `store`: Session repository used by local session operations.
    ///
    /// The service configured to persist session graphs.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_session_store(mut self, store: Arc<dyn scriberig_session::SessionStore>) -> Self {
        self.session_store = Some(store);
        self
    }

    /// Installs the safe model descriptors and selectable profile library.
    ///
    /// - `self`: Service being configured.
    /// - `library`: Provider-neutral model descriptors and profile references.
    ///
    /// The service configured with the supplied model library.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_model_library(mut self, library: ModelLibrary) -> Self {
        self.model_library = library;
        self
    }
}
