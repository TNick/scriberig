//! Resolves model choices from the narrowest execution scope outward.

use crate::{ApplicationError, SessionExecutionSelection};

/// Candidate model choices ordered from invocation scope to user scope.
///
/// # Public members
/// - Each field supplies an optional selection at the named scope.
///
/// # Private members
/// - None.
#[derive(Clone, Copy)]
pub struct ExecutionSelectionSources<'a> {
    /// One invocation-only override.
    pub invocation: Option<&'a SessionExecutionSelection>,
    /// A plan-node assignment scoped below invocation overrides.
    pub plan_node: Option<&'a SessionExecutionSelection>,
    /// A branch-specific default.
    pub branch: Option<&'a SessionExecutionSelection>,
    /// The active session selection.
    pub session: Option<&'a SessionExecutionSelection>,
    /// A project-level default.
    pub project: Option<&'a SessionExecutionSelection>,
    /// A user or model-profile default.
    pub user_profile: Option<&'a SessionExecutionSelection>,
}

/// Resolves the safe provider/model/thinking configuration for one plan node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPlanExecution {
    /// Upstream provider identity selected for this node.
    pub provider_id: String,
    /// Provider-scoped upstream model identity selected for this node.
    pub model_id: String,
    /// Whether this execution uses the host's configured runtime fallback.
    pub uses_runtime_fallback: bool,
    /// Advertised thinking capability selected for this invocation.
    pub thinking_level: Option<String>,
}

/// Resolves one node override before branch and session model defaults.
///
/// - `node`: Plan node whose optional assignment is resolved.
/// - `session`: Shared session default when the node has no override.
/// - `upstream_models`: Host-owned upstream model catalog.
/// - `runtime_fallback`: Runtime model used when no profile is selected.
///
/// Safe provider and model identities plus the resolved thinking level.
///
/// Returns an error for unavailable upstream provider/model pairs or thinking
/// values. Unsupported levels are clamped by the upstream model runtime.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_plan_node_execution(
    node: &scriberig_session::PlanNode,
    session: &SessionExecutionSelection,
    upstream_models: &[rpi_ai::Model],
    runtime_fallback: &str,
) -> Result<ResolvedPlanExecution, ApplicationError> {
    let plan_node = node
        .assignment
        .as_ref()
        .map(|assignment| SessionExecutionSelection {
            revision: 0,
            provider_id: Some(assignment.provider_id.clone()),
            model_id: Some(assignment.model_id.clone()),
            thinking_level: assignment.thinking_level.clone(),
            history: Vec::new(),
        });
    resolve_plan_node_execution_with_sources(
        node,
        ExecutionSelectionSources {
            invocation: None,
            plan_node: plan_node.as_ref(),
            branch: None,
            session: Some(session),
            project: None,
            user_profile: None,
        },
        upstream_models,
        runtime_fallback,
    )
}

/// Resolves a plan node against the full shared execution-selection hierarchy.
///
/// - `node`: Plan node whose safe assignment may override broader defaults.
/// - `sources`: Invocation, branch, session, project, and user selections.
/// - `upstream_models`: Host-owned upstream model catalog.
/// - `runtime_fallback`: Provider model used when no profile is selected.
///
/// Safe provider/model identities and the selected thinking level.
///
/// Returns an error when a selected upstream model or thinking id is invalid.
/// Valid but unsupported thinking is clamped upstream.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_plan_node_execution_with_sources(
    node: &scriberig_session::PlanNode,
    sources: ExecutionSelectionSources<'_>,
    upstream_models: &[rpi_ai::Model],
    runtime_fallback: &str,
) -> Result<ResolvedPlanExecution, ApplicationError> {
    let assignment = node
        .assignment
        .as_ref()
        .map(|assignment| SessionExecutionSelection {
            revision: 0,
            provider_id: Some(assignment.provider_id.clone()),
            model_id: Some(assignment.model_id.clone()),
            thinking_level: assignment.thinking_level.clone(),
            history: Vec::new(),
        });
    let resolved_sources = ExecutionSelectionSources {
        invocation: sources.invocation,
        plan_node: assignment.as_ref(),
        branch: sources.branch,
        session: sources.session,
        project: sources.project,
        user_profile: sources.user_profile,
    };
    let uses_runtime_fallback = [
        resolved_sources.invocation,
        resolved_sources.plan_node,
        resolved_sources.branch,
        resolved_sources.session,
        resolved_sources.project,
        resolved_sources.user_profile,
    ]
    .into_iter()
    .flatten()
    .all(|selection| selection.provider_id.is_none());
    let (provider_id, model_id, thinking_level) =
        resolve_execution_selection(resolved_sources, upstream_models, runtime_fallback)?;
    Ok(ResolvedPlanExecution {
        provider_id,
        model_id,
        uses_runtime_fallback,
        thinking_level,
    })
}

/// Resolves a model and canonical thinking level by scope priority.
///
/// Provider/model identity uses the first complete selection in invocation,
/// plan node, branch, session, project, and user-profile order. Thinking uses
/// the first canonical selection in that order. Model support and clamping
/// belong to the upstream runtime. An absent selection uses the runtime model.
///
/// - `sources`: Optional selections ordered by their semantic scope.
/// - `upstream_models`: Host-owned upstream model catalog.
/// - `runtime_fallback`: Provider model id used when no selection is present.
///
/// Upstream provider/model IDs and canonical thinking ID.
///
/// Returns an application error for unavailable or inconsistent selections.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_execution_selection(
    sources: ExecutionSelectionSources<'_>,
    upstream_models: &[rpi_ai::Model],
    runtime_fallback: &str,
) -> Result<(String, String, Option<String>), ApplicationError> {
    let ordered = [
        sources.invocation,
        sources.plan_node,
        sources.branch,
        sources.session,
        sources.project,
        sources.user_profile,
    ];
    let Some(selected) = ordered
        .into_iter()
        .flatten()
        .find(|selection| selection.provider_id.is_some())
    else {
        let (provider_id, model_id) = resolve_runtime_fallback(upstream_models, runtime_fallback)?;
        return Ok((provider_id, model_id, None));
    };
    let provider_id = selected.provider_id.as_deref().unwrap_or_default();
    let model_id = selected.model_id.as_deref().unwrap_or_default();
    let explicit_thinking = ordered
        .into_iter()
        .flatten()
        .find_map(|selection| selection.thinking_level.as_deref());
    let thinking_level = explicit_thinking
        .map(|level| super::model::ThinkingLevelId::new(level.to_owned()))
        .transpose()
        .map_err(invalid_selection)?
        .map(|level| level.as_str().to_owned());
    let requested_thinking = thinking_level
        .as_deref()
        .and_then(rpi_ai::ThinkingLevel::parse_name)
        .unwrap_or(rpi_ai::ThinkingLevel::Off);
    let resolved = rpi_model_runtime::resolve_selection(
        upstream_models,
        &rpi_model_runtime::ModelSelection {
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
            thinking: requested_thinking,
        },
    )
    .map_err(|error| invalid_selection(&error.to_string()))?;
    let model = &upstream_models[resolved.catalog_index];
    let effective_thinking =
        thinking_level.map(|_| resolved.effective_thinking.as_str().to_owned());
    Ok((model.provider.clone(), model.id.clone(), effective_thinking))
}

/// Resolves the unassigned runtime model through the upstream catalog.
///
/// The fallback carries no provider identity, so this finds its unique
/// provider in the catalog before delegating exact selection to upstream.
///
/// - `upstream_models`: Host-owned upstream model catalog.
/// - `runtime_fallback`: Provider-native model id configured by the host.
///
/// The canonical provider model id from the upstream catalog.
/// # Returns
/// The operation result.
/// # Errors
/// Returns invalid-request when the fallback is missing or provider-ambiguous.
fn resolve_runtime_fallback(
    upstream_models: &[rpi_ai::Model],
    runtime_fallback: &str,
) -> Result<(String, String), ApplicationError> {
    let providers = upstream_models
        .iter()
        .filter(|model| model.id.eq_ignore_ascii_case(runtime_fallback))
        .map(|model| model.provider.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if providers.len() != 1 {
        return Err(invalid_selection(
            "runtime fallback is missing or provider-ambiguous upstream",
        ));
    }
    let provider_id = providers.first().expect("one provider found");
    let resolved = rpi_model_runtime::resolve_selection(
        upstream_models,
        &rpi_model_runtime::ModelSelection {
            provider_id: (*provider_id).to_owned(),
            model_id: runtime_fallback.to_owned(),
            thinking: rpi_ai::ThinkingLevel::Off,
        },
    )
    .map_err(|error| invalid_selection(&error.to_string()))?;
    let model = &upstream_models[resolved.catalog_index];
    Ok((model.provider.clone(), model.id.clone()))
}

/// Creates a safe model-selection validation error.
///
/// - `message`: Safe explanation of the invalid selection.
///
/// An application invalid-request error.
/// # Returns
/// The operation result.
fn invalid_selection(message: &str) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "agent_start".to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
#[path = "resolution_tests.rs"]
mod tests;
