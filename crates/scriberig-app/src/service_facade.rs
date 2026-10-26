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
        }
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
