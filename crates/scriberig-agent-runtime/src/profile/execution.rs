//! Bridges `ScribeRig` profile preferences into shared app selection resolution.

use scriberig_agent_core::AgentProfileDescriptor;
use scriberig_app::{
    ApplicationError, ExecutionSelectionSources, ResolvedPlanExecution,
    resolve_execution_selection_with_profile_preference,
    resolve_plan_node_execution_with_profile_preference,
};
use scriberig_session::{PlanNode, SessionExecutionSelection};

/// Resolves a profile preference through the shared upstream-backed resolver.
///
/// Invocation, plan-node, branch, session, and project selections retain their
/// existing precedence. When none supplies a model, the exact profile's
/// preference is considered before the general user-profile fallback.
///
/// - `profile`: Exact `ScribeRig` profile descriptor for this execution.
/// - `sources`: Existing semantic execution assignments.
/// - `upstream_models`: Host-owned upstream catalog snapshot.
/// - `runtime_fallback`: Model selected by host configuration when unassigned.
///
/// Provider/model IDs and the upstream-resolved thinking level.
/// # Returns
/// The effective execution selection resolved against the host catalog.
/// # Errors
/// Returns an application error for unavailable model or thinking IDs.
pub fn resolve_profile_execution_selection(
    profile: &AgentProfileDescriptor,
    sources: ExecutionSelectionSources<'_>,
    upstream_models: &[rpi_ai::Model],
    runtime_fallback: &str,
) -> Result<(String, String, Option<rpi_ai::ThinkingLevel>), ApplicationError> {
    let selection = profile_execution_selection(profile);
    resolve_execution_selection_with_profile_preference(
        sources,
        selection.as_ref(),
        upstream_models,
        runtime_fallback,
    )
}

/// Resolves a plan node with its exact profile preference as a lower default.
///
/// The app resolver preserves a node's explicit assignment and all narrower
/// execution scopes before consulting the profile preference.
///
/// - `profile`: Exact `ScribeRig` profile descriptor for this execution.
/// - `node`: Durable plan node with an optional explicit assignment.
/// - `sources`: Existing branch, session, project, and user selections.
/// - `upstream_models`: Host-owned upstream catalog snapshot.
/// - `runtime_fallback`: Model selected by host configuration when unassigned.
///
/// The exact upstream model and resolved assignment for this worker.
/// # Returns
/// The plan-node execution resolved against the host catalog.
/// # Errors
/// Returns an application error for unavailable model or thinking IDs.
pub fn resolve_profile_plan_node_execution(
    profile: &AgentProfileDescriptor,
    node: &PlanNode,
    sources: ExecutionSelectionSources<'_>,
    upstream_models: &[rpi_ai::Model],
    runtime_fallback: &str,
) -> Result<ResolvedPlanExecution, ApplicationError> {
    let selection = profile_execution_selection(profile);
    resolve_plan_node_execution_with_profile_preference(
        node,
        sources,
        selection.as_ref(),
        upstream_models,
        runtime_fallback,
    )
}

/// Projects only public profile selection identifiers into app resolution.
///
/// The temporary value carries no profile revision, credentials, provider
/// configuration, or model metadata. The exact profile revision remains part
/// of context compilation and request provenance.
///
/// - `profile`: Profile whose preferred execution selection is projected.
///
/// A temporary session-selection shape for shared app resolution, if selected.
/// # Returns
/// `None` when the profile has no execution preference.
fn profile_execution_selection(
    profile: &AgentProfileDescriptor,
) -> Option<SessionExecutionSelection> {
    profile
        .preferred_execution
        .as_ref()
        .map(|preference| SessionExecutionSelection {
            revision: 0,
            provider_id: Some(preference.provider_id.clone()),
            model_id: Some(preference.model_id.clone()),
            thinking_level: preference.thinking_level.clone(),
            history: Vec::new(),
        })
}

#[cfg(test)]
mod tests {
    //! Verifies the profile preference bridge stays free of local model data.

    use super::*;
    use scriberig_agent_core::AgentProfileExecutionPreference;

    /// Verifies profile selection projection carries only stable upstream IDs.
    #[test]
    fn profile_preference_projects_to_app_selection_shape() {
        let mut profile = crate::profile::bootstrap_default_agent_profile();
        profile.preferred_execution = Some(AgentProfileExecutionPreference {
            provider_id: "provider-a".to_owned(),
            model_id: "model-a".to_owned(),
            thinking_level: Some("high".to_owned()),
        });

        assert_eq!(
            profile_execution_selection(&profile),
            Some(SessionExecutionSelection {
                revision: 0,
                provider_id: Some("provider-a".to_owned()),
                model_id: Some("model-a".to_owned()),
                thinking_level: Some("high".to_owned()),
                history: Vec::new(),
            })
        );
    }
}
