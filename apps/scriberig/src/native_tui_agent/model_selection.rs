//! Resolves persisted model selections for native agent invocations.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    ExecutionSelectionSources, SessionBackend, SessionRequest, resolve_execution_selection_typed,
};
use scriberig_core::EntityId;

/// Holds the exact upstream model and product selection derived together.
pub(super) struct ResolvedNativeModel {
    /// Exact catalog model selected for provider construction.
    pub model: rpi_ai::Model,
    /// Upstream-clamped `ScribeRig` thinking assignment.
    pub thinking_level: Option<rpi_ai::ThinkingLevel>,
    /// Host connection identity for credentials and compatibility paths.
    pub connection: String,
}

/// Resolves a session selection into native provider request settings.
///
/// - `service`: Shared application service owning model and session state.
/// - `context`: Authorized caller context.
/// - `session`: Session whose persisted selection applies to this invocation.
/// - `fallback_model`: Runtime model used when no profile is selected.
///
/// Upstream provider/model identity and the selected thinking id.
///
/// Returns a safe application error for invalid selections or an unsupported
/// provider connection.
pub(super) fn selected_runtime_model<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    fallback_model: &str,
) -> Result<ResolvedNativeModel, ApplicationError> {
    let saved = SessionBackend::read_session(service, context, &SessionRequest { session })?;
    let selection = saved.execution;
    let upstream_models = service.upstream_models();
    resolve_selected_runtime_model_with_model(&upstream_models, &selection, fallback_model)
}

/// Resolves an upstream model selection for the native provider.
///
/// - `upstream_models`: Upstream models used to validate the selection.
/// - `selection`: Immutable session execution selection for this invocation.
/// - `fallback_model`: Runtime model used when no profile is selected.
///
/// Upstream provider/model identity, thinking id, and native adapter identity.
///
/// Returns an application error for invalid selections or an unsupported
/// provider connection.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
#[cfg(test)]
pub(super) fn resolve_selected_runtime_model(
    upstream_models: &[rpi_ai::Model],
    selection: &scriberig_app::SessionExecutionSelection,
    fallback_model: &str,
) -> Result<(String, String, Option<String>, String), ApplicationError> {
    let resolved =
        resolve_selected_runtime_model_with_model(upstream_models, selection, fallback_model)?;
    Ok((
        resolved.model.provider,
        resolved.model.id,
        resolved
            .thinking_level
            .map(|level| level.as_str().to_owned()),
        resolved.connection,
    ))
}

/// Resolves a model and its session settings from one catalog snapshot.
///
/// - `upstream_models`: Upstream models used to validate the selection.
/// - `selection`: Immutable session execution selection for this invocation.
/// - `fallback_model`: Runtime model used when no profile is selected.
///
/// The exact resolved model, thinking assignment, and connection identity.
///
/// # Returns
/// The exact upstream model and effective session selections.
///
/// # Errors
/// Returns an application error for invalid selections.
pub(super) fn resolve_selected_runtime_model_with_model(
    upstream_models: &[rpi_ai::Model],
    selection: &scriberig_app::SessionExecutionSelection,
    fallback_model: &str,
) -> Result<ResolvedNativeModel, ApplicationError> {
    let (provider_id, model_id, thinking_level) = resolve_execution_selection_typed(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: None,
            session: Some(selection),
            project: None,
            user_profile: None,
        },
        upstream_models,
        fallback_model,
    )?;
    let model = upstream_models
        .iter()
        .find(|model| model.provider == provider_id && model.id == model_id)
        .cloned()
        .ok_or_else(|| ApplicationError::InvalidRequest {
            operation: "agent_start".to_owned(),
            message: "resolved model is missing from the upstream catalog".to_owned(),
        })?;
    let connection = if selection.provider_id.is_some() {
        super::provider_config::catalog::connection_identity(&provider_id, &model_id)
    } else {
        "local".to_owned()
    };
    Ok(ResolvedNativeModel {
        model,
        thinking_level,
        connection,
    })
}
