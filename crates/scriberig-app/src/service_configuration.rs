//! Provides optional application-service configuration boundaries.

use std::sync::Arc;

use crate::ApplicationService;

impl<B> ApplicationService<B> {
    /// Installs the shared semantic profile provider used by agent invocations.
    ///
    /// # Parameters
    /// - `self`: Service being configured.
    /// - `provider`: Provider backed by universal revision/snapshot/root storage.
    ///
    /// # Returns
    /// The service configured to resolve exact persisted profile revisions.
    #[must_use]
    pub fn with_agent_profile_provider(
        mut self,
        provider: Arc<dyn crate::AgentProfileProvider>,
    ) -> Self {
        self.agent_profile_provider = Some(provider);
        self
    }

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

    /// Installs shared projection style ownership for this service.
    ///
    /// - `self`: Service being configured.
    /// - `configuration`: Style adapter, configuration store, and convention sources.
    ///
    /// The service configured with the supplied projection style ownership.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_projection_style_configuration(
        mut self,
        configuration: crate::ProjectionStyleConfiguration,
    ) -> Self {
        self.projection_style = configuration;
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
        self.provider_extension_runtime
            .model_catalog()
            .reconcile_models(models);
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
        self.provider_extension_runtime =
            crate::ProviderExtensionRuntime::with_model_catalog(catalog);
        self
    }

    /// Installs shared provider-extension state across product application surfaces.
    ///
    /// This handle keeps the upstream model catalog and extension generation
    /// slot together, so all surfaces capture and publish the same pair.
    ///
    /// # Parameters
    /// - `self`: Service being configured.
    /// - `runtime`: Shared provider-extension runtime.
    ///
    /// # Returns
    /// The configured service using the shared runtime.
    #[must_use]
    pub fn with_provider_extension_runtime(
        mut self,
        runtime: crate::ProviderExtensionRuntime,
    ) -> Self {
        self.provider_extension_runtime = runtime;
        self
    }

    /// Returns the shared provider-extension state handle.
    ///
    /// Product compositions should pass this handle to every service instance
    /// that must share catalog reloads and extension-session generations.
    ///
    /// # Returns
    /// A cloneable provider-extension runtime handle.
    #[must_use]
    pub fn provider_extension_runtime(&self) -> crate::ProviderExtensionRuntime {
        self.provider_extension_runtime.clone()
    }

    /// The upstream model catalog used for selection resolution.
    ///
    /// # Returns
    /// Model rows from one coherent catalog revision.
    #[must_use]
    pub fn upstream_models(&self) -> Vec<rpi_ai::Model> {
        self.provider_extension_runtime
            .model_catalog()
            .snapshot()
            .models
    }

    /// Returns a shared handle to the upstream live model catalog.
    ///
    /// - `self`: Service owning the catalog handle.
    ///
    /// A cloneable handle observing the same catalog state.
    ///
    /// # Returns
    /// The shared upstream model catalog.
    #[must_use]
    pub fn model_catalog(&self) -> rpi_model_runtime::ModelCatalog {
        self.provider_extension_runtime.model_catalog()
    }
}
