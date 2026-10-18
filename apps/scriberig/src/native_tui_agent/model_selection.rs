//! Resolves persisted model selections for native agent invocations.

use scriberig_app::{
    AgentConsoleBackend, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService, ExecutionSelectionSources, ModelLibrary, SessionBackend, SessionRequest,
    resolve_execution_selection,
};
use scriberig_core::EntityId;

/// Resolves a session selection into native provider request settings.
///
/// # Parameters
/// - `service`: Shared application service owning model and session state.
/// - `context`: Authorized caller context.
/// - `session`: Session whose persisted selection applies to this invocation.
/// - `fallback_model`: Runtime model used when no profile is selected.
///
/// # Returns
/// Provider model identifier and the selected thinking id when available.
///
/// # Errors
/// Returns a safe application error for invalid selections or an unsupported
/// provider connection.
pub(super) fn selected_runtime_model<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    fallback_model: &str,
) -> Result<(String, Option<String>), ApplicationError> {
    let saved = SessionBackend::read_session(service, context, &SessionRequest { session })?;
    let selection = saved.execution;
    if selection.model_profile.is_none() {
        return Ok((fallback_model.to_owned(), None));
    }
    let library = AgentConsoleBackend::agent_model_library(service, context)?;
    resolve_selected_runtime_model(&library, &selection, fallback_model)
}

/// Resolves a validated model-library selection for the native provider.
///
/// # Parameters
/// - `library`: Safe model and profile descriptors.
/// - `selection`: Immutable session execution selection for this invocation.
/// - `fallback_model`: Runtime model used when no profile is selected.
///
/// # Returns
/// Provider model identifier and model-advertised thinking id, when selected.
///
/// # Errors
/// Returns an application error for invalid selections or an unsupported
/// provider connection.
pub(super) fn resolve_selected_runtime_model(
    library: &ModelLibrary,
    selection: &scriberig_app::SessionExecutionSelection,
    fallback_model: &str,
) -> Result<(String, Option<String>), ApplicationError> {
    let (provider_model, thinking_level) = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: None,
            branch: None,
            session: Some(selection),
            project: None,
            user_profile: None,
        },
        library,
        fallback_model,
    )?;
    let Some(profile_id) = selection.model_profile.as_deref() else {
        return Ok((provider_model, thinking_level));
    };
    let profile = library
        .profile(profile_id)
        .filter(|profile| profile.enabled)
        .ok_or_else(|| ApplicationError::InvalidRequest {
            operation: "agent_start".to_owned(),
            message: "selected model profile is unavailable".to_owned(),
        })?;
    let model = library
        .model(&profile.model)
        .ok_or_else(|| ApplicationError::Internal {
            message: "selected model profile has no model descriptor".to_owned(),
        })?;
    if selection
        .model
        .as_deref()
        .is_some_and(|selected| selected != model.id.as_str())
    {
        return Err(ApplicationError::InvalidRequest {
            operation: "agent_start".to_owned(),
            message: "selected model profile no longer matches the session model".to_owned(),
        });
    }
    if profile.connection != "local" || model.provider_profile != "local" {
        return Err(ApplicationError::UnsupportedOperation {
            operation: "selected model profile uses an unavailable provider connection".to_owned(),
        });
    }
    Ok((provider_model, thinking_level))
}
