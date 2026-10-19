//! Implements captured model-request reads for remote TUI sessions.

use super::{
    ApplicationContext, ApplicationError, EntityId, RemoteAdapter, TuiModelRequestService,
    service_remote_models,
};

impl<T: scriberig_protocol::ProtocolTransport> TuiModelRequestService for RemoteAdapter<T> {
    /// Reads shared presentation defaults through the remote protocol client.
    ///
    /// - `self`: Adapter holding the protocol client.
    /// - `_context`: Locally supplied context, not serialized remotely.
    ///
    /// Revisioned shared preferences.
    ///
    /// Returns a sanitized remote protocol error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_presentation_preferences(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
        self.client
            .agent_presentation_preferences()
            .map_err(super::service_remote_mapping::remote_error)
    }

    /// Reads captured bounded request detail through the public client.
    ///
    /// - `_context`: Authorized application context.
    /// - `session`: Session containing the invocation.
    /// - `branch`: Branch containing the invocation.
    /// - `invocation`: Stable model-invocation identity.
    ///
    /// Captured provider-neutral request detail.
    ///
    /// Returns a sanitized remote protocol error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_request_detail(
        &self,
        _context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
        invocation: EntityId,
    ) -> Result<scriberig_protocol::ModelRequestDetailResponse, ApplicationError> {
        service_remote_models::model_request_detail(self, session, branch, invocation)
    }
}
