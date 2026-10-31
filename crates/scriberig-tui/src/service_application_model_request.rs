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
    let mut fragments = Vec::new();
    let mut offset = 0;
    let mut page = scriberig_app::agent_console::request_page::model_request_detail_page(
        &detail, offset, 32_768,
    );
    let total_chars = page.total_chars;
    let content_complete = page.content_complete;
    loop {
        let next_offset = page.next_offset;
        fragments.extend(page.fragments.into_iter().map(|fragment| {
            scriberig_protocol::ModelRequestDetailFragment {
                kind: fragment.kind,
                index: fragment.index,
                parent_index: fragment.parent_index,
                field: fragment.field,
                field_offset: fragment.field_offset,
                role: fragment.role,
                text: fragment.text,
            }
        }));
        match next_offset {
            Some(next) => {
                offset = next;
                page = scriberig_app::agent_console::request_page::model_request_detail_page(
                    &detail, offset, 32_768,
                );
            }
            None => break,
        }
    }
    scriberig_protocol::ModelRequestDetailResponse {
        invocation: invocation.to_string(),
        model: detail.model,
        provider_id: detail.provider_id,
        provider_profile: detail.provider_profile,
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
        context_fingerprint: detail.context_fingerprint,
        request_fingerprint: detail.request_fingerprint,
        catalog_revision: detail.catalog_revision,
        upstream_runtime_revision: detail.upstream_runtime_revision,
        provider_response_id: detail.provider_response_id,
        model_execution_configuration: detail.model_execution_configuration,
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
        applied_settings: detail.applied_settings,
        applied_settings_fingerprint: detail.applied_settings_fingerprint,
        truncated: detail.truncated,
        content_complete,
        fragments,
        offset: Some(0),
        total_chars: Some(total_chars),
        next_offset: None,
    }
}
