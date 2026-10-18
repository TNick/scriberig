//! Reads captured model requests through a shared application adapter.

use super::{
    ApplicationAdapter, ApplicationContext, ApplicationError, EntityId, TuiModelRequestService,
};
use scriberig_app::AgentConsoleBackend;

impl<S: AgentConsoleBackend> TuiModelRequestService for ApplicationAdapter<S> {
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
        let snapshot = self.service.agent_presentation_preferences(context)?;
        Ok(super::service_remote_mapping::presentation_response(
            &snapshot,
        ))
    }

    /// Projects a captured provider-neutral model request into protocol detail.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    /// - `session`: Session containing the invocation.
    /// - `branch`: Branch containing the invocation.
    /// - `invocation`: Stable model-invocation identity.
    ///
    /// # Returns
    /// Bounded captured request detail.
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
        let detail = self
            .service
            .model_request_detail(context, session, branch, invocation)?;
        Ok(model_request_response(invocation, detail))
    }
}

/// Converts a shared request projection into a safe protocol response.
///
/// # Parameters
/// - `invocation`: Stable model-invocation identity.
/// - `detail`: Captured provider-neutral request projection.
///
/// # Returns
/// Bounded protocol model-request detail.
pub(super) fn model_request_response(
    invocation: EntityId,
    detail: scriberig_session::ModelRequestProjection,
) -> scriberig_protocol::ModelRequestDetailResponse {
    scriberig_protocol::ModelRequestDetailResponse {
        invocation: invocation.to_string(),
        model: detail.model,
        thinking_level: detail.thinking_level,
        messages: detail
            .messages
            .into_iter()
            .map(|message| scriberig_protocol::ModelRequestMessageDetail {
                role: message.role,
                content: message.content,
            })
            .collect(),
        tools: detail
            .tools
            .into_iter()
            .map(|tool| scriberig_protocol::ModelRequestToolDetail {
                name: tool.name,
                description: tool.description,
                parameters: tool.parameters,
            })
            .collect(),
        truncated: detail.truncated,
    }
}
