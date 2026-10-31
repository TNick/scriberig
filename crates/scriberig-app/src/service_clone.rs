//! Clones the shared application facade without duplicating its state.

use super::ApplicationService;

impl<B> Clone for ApplicationService<B> {
    /// Clones the application facade while sharing mutable application state.
    ///
    /// # Parameters
    /// - `self`: Application facade to clone.
    ///
    /// # Returns
    /// A facade over the same backend, operation registry, sessions, and stores.
    fn clone(&self) -> Self {
        Self {
            backend: std::sync::Arc::clone(&self.backend),
            operations: self.operations.clone(),
            sessions: std::sync::Arc::clone(&self.sessions),
            next_agent_handle: std::sync::Arc::clone(&self.next_agent_handle),
            session_store: self.session_store.clone(),
            agent_profile_provider: self.agent_profile_provider.clone(),
            provider_extension_runtime: self.provider_extension_runtime.clone(),
            extension_component_mailbox: self.extension_component_mailbox.clone(),
            presentation_preferences: std::sync::Arc::clone(&self.presentation_preferences),
            projection_style: self.projection_style.clone(),
            project_service: self.project_service.clone(),
            cad_provider_registry: std::sync::Arc::clone(&self.cad_provider_registry),
        }
    }
}
