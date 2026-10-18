//! Reads captured model requests through an application operation adapter.

use super::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationOperationAdapter,
    EntityId, TuiModelRequestService,
};

impl<B: ApplicationBackend> TuiModelRequestService for ApplicationOperationAdapter<B> {
    /// Reads shared presentation defaults through the application service.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    ///
    /// # Returns
    /// Revisioned provider-neutral preferences.
    ///
    /// # Errors
    /// Returns the application authorization or storage error.
    fn agent_presentation_preferences(
        &self,
        context: &ApplicationContext,
    ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
        let snapshot = scriberig_app::AgentConsoleBackend::agent_presentation_preferences(
            &self.service,
            context,
        )?;
        Ok(super::service_remote_mapping::presentation_response(
            &snapshot,
        ))
    }

    /// Reads captured safe request detail through the shared application port.
    ///
    /// # Parameters
    /// - `context`: Authorized application context.
    /// - `session`: Session containing the invocation.
    /// - `branch`: Branch containing the invocation.
    /// - `invocation`: Stable model-invocation identity.
    ///
    /// # Returns
    /// Bounded provider-neutral request detail.
    ///
    /// # Errors
    /// Returns the application authorization or lookup error.
    fn model_request_detail(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
        invocation: EntityId,
    ) -> Result<scriberig_protocol::ModelRequestDetailResponse, ApplicationError> {
        let detail = scriberig_app::AgentConsoleBackend::model_request_detail(
            &self.service,
            context,
            session,
            branch,
            invocation,
        )?;
        Ok(super::service_application_model_request::model_request_response(invocation, detail))
    }
}
