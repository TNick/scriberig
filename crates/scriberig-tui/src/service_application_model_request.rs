//! Reads captured model requests through a shared application adapter.

use super::{
    ApplicationAdapter, ApplicationContext, ApplicationError, EntityId, TuiModelRequestService,
};
use scriberig_app::AgentConsoleBackend;

impl<S: AgentConsoleBackend> TuiModelRequestService for ApplicationAdapter<S> {
    /// Reads shared presentation defaults through the application service.
    ///
    /// - `context`: Caller authorization context.
    ///
    /// Revisioned provider-neutral preferences.
    ///
    /// Returns the application authorization or storage error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `context`: Caller authorization context.
    /// - `session`: Session containing the invocation.
    /// - `branch`: Branch containing the invocation.
    /// - `invocation`: Stable model-invocation identity.
    ///
    /// Bounded captured request detail.
    ///
    /// Returns the application authorization or lookup error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
/// - `invocation`: Stable model-invocation identity.
/// - `detail`: Captured provider-neutral request projection.
///
/// Bounded protocol model-request detail.
/// # Returns
/// The operation result.
pub(super) fn model_request_response(
    invocation: EntityId,
    detail: scriberig_session::ModelRequestProjection,
) -> scriberig_protocol::ModelRequestDetailResponse {
    scriberig_protocol::ModelRequestDetailResponse {
        invocation: invocation.to_string(),
        model: detail.model,
        provider_id: detail.provider_id,
        thinking_level: detail.thinking_level,
        profile: detail.profile.map(|reference| {
            scriberig_protocol::ModelContextRevisionReference {
                identity: reference.identity,
                revision: reference.revision,
            }
        }),
        active_components: detail
            .active_components
            .into_iter()
            .map(
                |reference| scriberig_protocol::ModelContextRevisionReference {
                    identity: reference.identity,
                    revision: reference.revision,
                },
            )
            .collect(),
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
