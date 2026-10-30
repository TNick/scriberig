//! Constructs and exposes the shared application service facade.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use super::ApplicationService;

impl<B> ApplicationService<B> {
    /// Creates an application service around canonical subsystem providers.
    ///
    /// # Parameters
    /// - `backend`: Backend implementing universal operation execution.
    ///
    /// # Returns
    /// A transport-independent application service.
    ///
    /// # Returns
    /// An application service with the shared runtime subsystems initialized.
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
            operations: crate::OperationRegistry::new(),
            sessions: Arc::new(Mutex::new(BTreeMap::new())),
            next_agent_handle: Arc::new(AtomicU64::new(1)),
            session_store: None,
            agent_profile_provider: None,
            provider_extension_runtime: crate::ProviderExtensionRuntime::new(),
            extension_component_mailbox: None,
            presentation_preferences: Arc::new(
                crate::InMemoryAgentPresentationPreferenceStore::default(),
            ),
            projection_style: crate::ProjectionStyleConfiguration::default(),
            project_service: None,
            cad_provider_registry: Arc::new(
                crate::cad_provider_registry::CadProviderRegistry::new(),
            ),
        }
    }

    /// Injects optional external CAD providers selected by the product host.
    ///
    /// # Parameters
    /// - `registry`: Registry containing initialized providers and safe failure states.
    ///
    /// # Returns
    /// The application service configured to report and use this provider registry.
    #[must_use]
    pub fn with_cad_provider_registry(
        mut self,
        registry: crate::cad_provider_registry::CadProviderRegistry,
    ) -> Self {
        self.cad_provider_registry = Arc::new(registry.clone());
        if let Some(project_service) = self.project_service.take() {
            self.project_service = Some(Arc::new(
                (*project_service)
                    .clone()
                    .with_cad_provider_registry(registry),
            ));
        }
        self
    }

    /// Reports one configured provider status for a CAD format.
    ///
    /// # Parameters
    /// - `format`: External CAD format to inspect.
    ///
    /// # Returns
    /// Provider-neutral availability, capability, and fidelity metadata.
    #[must_use]
    pub fn cad_provider_status(
        &self,
        format: &scriberig_domain_cad::CadFormat,
    ) -> scriberig_domain_cad::CadProviderStatus {
        self.cad_provider_registry.status(format)
    }

    /// Lists configured CAD provider statuses for runtime capability discovery.
    ///
    /// # Returns
    /// Stable provider-neutral statuses, including explicit DWG unavailability.
    #[must_use]
    pub fn cad_provider_statuses(&self) -> Vec<scriberig_domain_cad::CadProviderStatus> {
        self.cad_provider_registry.statuses()
    }

    /// Attaches the shared persisted project catalog used for artifact reads.
    ///
    /// # Parameters
    /// - `project_service`: Project service over the shared durable catalog.
    ///
    /// The application service configured to expose authorized project reads.
    ///
    /// # Returns
    /// The configured application service.
    #[must_use]
    pub fn with_project_service(
        mut self,
        project_service: scriberig_project::ProjectService,
    ) -> Self {
        self.project_service = Some(Arc::new(
            project_service.with_cad_provider_registry((*self.cad_provider_registry).clone()),
        ));
        self
    }

    /// Clones the configured project service for a product adapter.
    ///
    /// Product adapters that operate on project stores must request the service
    /// through this method so import, fidelity, reads, and candidates share the
    /// same injected domain providers.
    ///
    /// # Parameters
    /// # Returns
    /// The configured project service, when one was attached.
    #[must_use]
    pub fn configured_project_service(&self) -> Option<scriberig_project::ProjectService> {
        self.project_service
            .as_ref()
            .map(|service| (**service).clone())
    }

    /// Supplies the session-scoped upstream plugin component registry.
    ///
    /// # Parameters
    /// - `mailbox`: Shared registry consumed by the product TUI.
    ///
    /// # Returns
    /// The service configured to route isolated plugin component actions.
    #[must_use]
    pub fn with_extension_component_mailbox(
        mut self,
        mailbox: rpi_extensions::ExtensionStatusMailbox,
    ) -> Self {
        self.extension_component_mailbox = Some(mailbox);
        self
    }

    /// Returns the upstream plugin component registry, if configured.
    ///
    /// # Returns
    /// A clone sharing the configured mailbox state.
    #[must_use]
    pub fn extension_component_mailbox(&self) -> Option<rpi_extensions::ExtensionStatusMailbox> {
        self.extension_component_mailbox.clone()
    }

    /// Returns the injected backend by shared reference.
    ///
    /// # Parameters
    /// - `self`: Application service to inspect.
    ///
    /// # Returns
    /// The backend used for semantic execution.
    #[must_use]
    pub fn backend(&self) -> &B {
        self.backend.as_ref()
    }

    /// Returns the lifecycle registry used by this application service.
    ///
    /// # Parameters
    /// - `self`: Application service to inspect.
    ///
    /// # Returns
    /// The shared operation lifecycle registry.
    #[must_use]
    pub const fn operations(&self) -> &crate::OperationRegistry {
        &self.operations
    }
}
