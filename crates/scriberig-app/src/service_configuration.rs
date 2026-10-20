//! Provides optional application-service configuration boundaries.

use std::sync::Arc;

use crate::ApplicationService;

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

    /// Seeds a new live catalog from legacy host-owned model rows.
    ///
    /// - `self`: Service being configured.
    /// - `models`: Upstream model catalog, including request-only metadata.
    ///
    /// This compatibility builder is intended for static callers; use
    /// `with_model_catalog` when multiple components must observe refreshes.
    ///
    /// # Returns
    /// The configured service with the supplied model catalog.
    #[must_use]
    pub fn with_upstream_models(self, models: Vec<rpi_ai::Model>) -> Self {
        self.model_catalog.reconcile_models(models);
        self
    }

    /// Installs a shared live upstream model catalog.
    ///
    /// - `self`: Service being configured.
    /// - `catalog`: Shared upstream catalog handle.
    ///
    /// The service configured to resolve reads and selections from `catalog`.
    ///
    /// # Returns
    /// The configured service.
    #[must_use]
    pub fn with_model_catalog(mut self, catalog: rpi_model_runtime::ModelCatalog) -> Self {
        self.model_catalog = catalog;
        self
    }
}
