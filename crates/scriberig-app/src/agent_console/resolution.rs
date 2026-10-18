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
    /// A branch-specific default.
    pub branch: Option<&'a SessionExecutionSelection>,
    /// The active session selection.
    pub session: Option<&'a SessionExecutionSelection>,
    /// A project-level default.
    pub project: Option<&'a SessionExecutionSelection>,
    /// A user or model-profile default.
    pub user_profile: Option<&'a SessionExecutionSelection>,
}

/// Resolves a model and thinking id by scope priority and model support.
///
/// Model/profile identity uses the first complete selection in invocation,
/// branch, session, project, and user-profile order. Thinking uses the first
/// advertised level in that same order, then profile/model defaults, then the
/// model's first advertised level. An absent selection uses the runtime model.
///
/// # Parameters
/// - `sources`: Optional selections ordered by their semantic scope.
/// - `library`: Validated safe model/profile library.
/// - `runtime_fallback`: Provider model id used when no profile is selected.
///
/// # Returns
/// Provider-native model id and supported thinking id, when available.
///
/// # Errors
/// Returns an application error for unavailable or inconsistent profiles.
pub fn resolve_execution_selection(
    sources: ExecutionSelectionSources<'_>,
    library: &ModelLibrary,
    runtime_fallback: &str,
) -> Result<(String, Option<String>), ApplicationError> {
    let ordered = [
        sources.invocation,
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
    let thinking_level = ordered
        .into_iter()
        .flatten()
        .filter_map(|selection| selection.thinking_level.as_deref())
        .find(|level| supports(level))
        .map(str::to_owned)
        .or_else(|| supported_default(profile, model, supports));
    Ok((model.provider_model_id.clone(), thinking_level))
}

/// Resolves the profile, model, then advertised-list thinking default.
///
/// # Parameters
/// - `profile`: Selected profile with optional default.
/// - `model`: Selected model descriptor.
/// - `supports`: Predicate checking model-advertised thinking ids.
///
/// # Returns
/// The first supported profile/model default, or first advertised level.
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
/// # Parameters
/// - `message`: Safe explanation of the invalid selection.
///
/// # Returns
/// An application invalid-request error.
fn invalid_selection(message: &str) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "agent_start".to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
#[path = "resolution_tests.rs"]
mod tests;
