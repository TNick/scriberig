//! Resolves model choices from the narrowest execution scope outward.

use super::model::{ModelLibrary, ModelProfile};
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
    /// Curated profile selected for this node.
    pub model_profile: Option<String>,
    /// Safe provider configuration identity, without credentials.
    pub provider_profile: Option<String>,
    /// Stable model identity selected for the node.
    pub model: Option<String>,
    /// Provider-native model identifier for invocation setup.
    pub provider_model_id: String,
    /// Advertised thinking capability selected for this invocation.
    pub thinking_level: Option<String>,
}

/// Resolves one node override before branch and session model defaults.
///
/// - `node`: Plan node whose optional assignment is resolved.
/// - `session`: Shared session default when the node has no override.
/// - `library`: Existing validated model-library architecture.
/// - `runtime_fallback`: Runtime model used when no profile is selected.
///
/// Safe provider and model identities plus the resolved thinking level.
///
/// Returns an error for unavailable profiles, mismatched models, or an
/// unsupported explicitly assigned thinking level.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_plan_node_execution(
    node: &scriberig_session::PlanNode,
    session: &SessionExecutionSelection,
    library: &ModelLibrary,
    runtime_fallback: &str,
) -> Result<ResolvedPlanExecution, ApplicationError> {
    let plan_node = node
        .assignment
        .as_ref()
        .map(|assignment| SessionExecutionSelection {
            revision: 0,
            model_profile: Some(assignment.profile.clone()),
            model: Some(assignment.model.clone()),
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
        library,
        runtime_fallback,
    )
}

/// Resolves a plan node against the full shared execution-selection hierarchy.
///
/// - `node`: Plan node whose safe assignment may override broader defaults.
/// - `sources`: Invocation, branch, session, project, and user selections.
/// - `library`: Existing validated model library.
/// - `runtime_fallback`: Provider model used when no profile is selected.
///
/// Safe provider/model identities and the selected thinking level.
///
/// Returns an error when a selected profile, model, or thinking level is invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_plan_node_execution_with_sources(
    node: &scriberig_session::PlanNode,
    sources: ExecutionSelectionSources<'_>,
    library: &ModelLibrary,
    runtime_fallback: &str,
) -> Result<ResolvedPlanExecution, ApplicationError> {
    let assignment = node
        .assignment
        .as_ref()
        .map(|assignment| SessionExecutionSelection {
            revision: 0,
            model_profile: Some(assignment.profile.clone()),
            model: Some(assignment.model.clone()),
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
    let (provider_model_id, thinking_level) =
        resolve_execution_selection(resolved_sources, library, runtime_fallback)?;
    let selected_profile = [
        resolved_sources.invocation,
        resolved_sources.plan_node,
        resolved_sources.branch,
        resolved_sources.session,
        resolved_sources.project,
        resolved_sources.user_profile,
    ]
    .into_iter()
    .flatten()
    .find(|selection| selection.model_profile.is_some())
    .and_then(|selection| selection.model_profile.as_deref());
    let selected_profile = selected_profile.and_then(|profile| library.profile(profile));
    let selected_model = selected_profile.and_then(|profile| library.model(&profile.model));
    Ok(ResolvedPlanExecution {
        model_profile: selected_profile.map(|profile| profile.id.clone()),
        provider_profile: selected_profile.map(|profile| profile.connection.clone()),
        model: selected_model.map(|model| model.id.as_str().to_owned()),
        provider_model_id,
        thinking_level,
    })
}

/// Resolves a model and thinking id by scope priority and model support.
///
/// Model/profile identity uses the first complete selection in invocation,
/// plan node, branch, session, project, and user-profile order. Thinking uses the first
/// advertised level in that same order, then profile/model defaults, then the
/// model's first advertised level. An absent selection uses the runtime model.
///
/// - `sources`: Optional selections ordered by their semantic scope.
/// - `library`: Validated safe model/profile library.
/// - `runtime_fallback`: Provider model id used when no profile is selected.
///
/// Provider-native model id and supported thinking id, when available.
///
/// Returns an application error for unavailable or inconsistent profiles.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_execution_selection(
    sources: ExecutionSelectionSources<'_>,
    library: &ModelLibrary,
    runtime_fallback: &str,
) -> Result<(String, Option<String>), ApplicationError> {
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
        .find(|selection| selection.model_profile.is_some())
    else {
        return Ok((runtime_fallback.to_owned(), None));
    };
    let profile_id = selected.model_profile.as_deref().unwrap_or_default();
    let profile = library
        .profile(profile_id)
        .filter(|profile| profile.enabled)
        .ok_or_else(|| invalid_selection("selected model profile is unavailable"))?;
    let model = library
        .model(&profile.model)
        .ok_or_else(|| ApplicationError::Internal {
            message: "selected model profile has no model descriptor".to_owned(),
        })?;
    if selected
        .model
        .as_deref()
        .is_some_and(|model_id| model_id != model.id.as_str())
    {
        return Err(invalid_selection(
            "selected model profile no longer matches its model",
        ));
    }
    let supports = |level: &str| {
        model
            .thinking_levels
            .iter()
            .any(|candidate| candidate.id.as_str() == level)
    };
    let explicit_thinking = ordered
        .into_iter()
        .flatten()
        .find_map(|selection| selection.thinking_level.as_deref());
    if explicit_thinking.is_some_and(|level| !supports(level)) {
        return Err(invalid_selection(
            "selected thinking level is not advertised by its model",
        ));
    }
    let thinking_level = explicit_thinking
        .map(str::to_owned)
        .or_else(|| supported_default(profile, model, supports));
    Ok((model.provider_model_id.clone(), thinking_level))
}

/// Resolves the profile, model, then advertised-list thinking default.
///
/// - `profile`: Selected profile with optional default.
/// - `model`: Selected model descriptor.
/// - `supports`: Predicate checking model-advertised thinking ids.
///
/// The first supported profile/model default, or first advertised level.
/// # Returns
/// The operation result.
fn supported_default(
    profile: &ModelProfile,
    model: &super::model::ModelDescriptor,
    supports: impl Fn(&str) -> bool,
) -> Option<String> {
    profile
        .default_thinking_level
        .as_ref()
        .filter(|level| supports(level.as_str()))
        .map(|level| level.as_str().to_owned())
        .or_else(|| {
            model
                .default_thinking_level
                .as_ref()
                .filter(|level| supports(level.as_str()))
                .map(|level| level.as_str().to_owned())
        })
        .or_else(|| {
            model
                .thinking_levels
                .first()
                .map(|level| level.id.as_str().to_owned())
        })
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
