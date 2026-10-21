//! Resolves persisted model selections for native agent invocations.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    ExecutionSelectionSources, SessionBackend, SessionRequest, resolve_execution_selection,
};
use scriberig_core::EntityId;

/// Resolves a session selection into native provider request settings.
///
/// - `service`: Shared application service owning model and session state.
/// - `context`: Authorized caller context.
/// - `session`: Session whose persisted selection applies to this invocation.
/// - `fallback_model`: Runtime model used when no profile is selected.
///
/// Provider model identifier and the selected thinking id when available.
///
/// Returns a safe application error for invalid selections or an unsupported
/// provider connection.
pub(super) fn selected_runtime_model<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    fallback_model: &str,
) -> Result<(String, Option<String>, String), ApplicationError> {
    let saved = SessionBackend::read_session(service, context, &SessionRequest { session })?;
    let selection = saved.execution;
    resolve_selected_runtime_model(service.upstream_models(), &selection, fallback_model)
}

/// Resolves an upstream model selection for the native provider.
///
/// - `upstream_models`: Upstream models used to validate the selection.
/// - `selection`: Immutable session execution selection for this invocation.
/// - `fallback_model`: Runtime model used when no profile is selected.
///
/// Provider model identifier, thinking id, and native adapter identity.
///
/// Returns an application error for invalid selections or an unsupported
/// provider connection.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn resolve_selected_runtime_model(
    upstream_models: &[rpi_ai::Model],
    selection: &scriberig_app::SessionExecutionSelection,
    fallback_model: &str,
) -> Result<(String, Option<String>, String), ApplicationError> {
    let (provider_id, model_id, thinking_level) = resolve_execution_selection(
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
    let connection = if selection.provider_id.is_some() {
        super::provider_config::catalog::connection_identity(&provider_id, &model_id)
    } else {
        "local".to_owned()
    };
    Ok((model_id, thinking_level, connection))
}
