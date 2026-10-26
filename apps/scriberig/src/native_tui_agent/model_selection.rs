//! Resolves persisted model selections for native agent invocations.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    ExecutionSelectionSources, SessionBackend, SessionRequest,
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
    /// Exact profile revision selected for this root invocation.
    pub profile: scriberig_app::AgentProfileDescriptor,
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
    let profile = service
        .resolve_agent_profile(
            context,
            scriberig_agent_runtime::profile::DEFAULT_AGENT_PROFILE_IDENTITY,
        )
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "agent_start".to_owned(),
            message: error.to_string(),
        })?
        .unwrap_or_else(scriberig_agent_runtime::profile::bootstrap_default_agent_profile);
    let upstream_models = service.upstream_models();
    resolve_selected_runtime_model_with_profile(
        &upstream_models,
        &selection,
        Some(profile),
        fallback_model,
    )
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
    resolve_selected_runtime_model_with_profile(upstream_models, selection, None, fallback_model)
}

/// Resolves native model settings using the exact active profile preference.
///
/// - `upstream_models`: Upstream models used to validate the selection.
/// - `selection`: Immutable session execution selection for this invocation.
/// - `profile`: Exact semantic profile resolved for this invocation.
/// - `fallback_model`: Runtime model used when no selection is present.
///
/// The exact model, effective thinking level, connection, and profile.
///
/// # Returns
/// The resolved native invocation configuration.
///
/// # Errors
/// Returns an error when a configured selection is unavailable.
fn resolve_selected_runtime_model_with_profile(
    upstream_models: &[rpi_ai::Model],
    selection: &scriberig_app::SessionExecutionSelection,
    profile: Option<scriberig_app::AgentProfileDescriptor>,
    fallback_model: &str,
) -> Result<ResolvedNativeModel, ApplicationError> {
    let profile_selection = profile
        .as_ref()
        .and_then(|profile| profile.preferred_execution.as_ref())
        .map(|preference| scriberig_app::SessionExecutionSelection {
            revision: 0,
            provider_id: Some(preference.provider_id.clone()),
            model_id: Some(preference.model_id.clone()),
            thinking_level: preference.thinking_level.clone(),
            history: Vec::new(),
        });
    let (provider_id, model_id, thinking_level) =
        scriberig_app::resolve_execution_selection_with_profile_preference(
            ExecutionSelectionSources {
                invocation: None,
                plan_node: None,
                branch: None,
                session: Some(selection),
                project: None,
                user_profile: None,
            },
            profile_selection.as_ref(),
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
    let connection = if selection.provider_id.is_some() || profile_selection.is_some() {
        super::provider_config::catalog::connection_identity(&provider_id, &model_id)
    } else {
        "local".to_owned()
    };
    Ok(ResolvedNativeModel {
        model,
        thinking_level,
        connection,
        profile: profile
            .unwrap_or_else(scriberig_agent_runtime::profile::bootstrap_default_agent_profile),
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_selected_runtime_model_with_profile;
    use scriberig_app::{AgentProfileDescriptor, AgentProfileExecutionPreference};
    use scriberig_session::SessionExecutionSelection;

    /// Uses the profile's upstream preference when no narrower choice exists.
    #[test]
    fn profile_preference_selects_native_model_and_connection() {
        let models = vec![
            rpi_ai::Model::new(
                "profile-model",
                "Profile Model",
                rpi_ai::Api::OpenaiCompletions,
                "profile-provider",
                "https://profile.example.test",
            ),
            rpi_ai::Model::new(
                "fallback-model",
                "Fallback Model",
                rpi_ai::Api::OpenaiCompletions,
                "fallback-provider",
                "https://fallback.example.test",
            ),
        ];
        let profile = AgentProfileDescriptor {
            reference: scriberig_app::AgentProfileRevisionRef {
                identity: "profile-test".to_owned(),
                revision: "revision-test".to_owned(),
            },
            name: "Profile test".to_owned(),
            components: Vec::new(),
            preferred_execution: Some(AgentProfileExecutionPreference {
                provider_id: "profile-provider".to_owned(),
                model_id: "profile-model".to_owned(),
                thinking_level: None,
            }),
        };
        let resolved = resolve_selected_runtime_model_with_profile(
            &models,
            &SessionExecutionSelection::default(),
            Some(profile),
            "fallback-model",
        )
        .expect("profile preference resolves");

        assert_eq!(resolved.model.provider, "profile-provider");
        assert_eq!(resolved.model.id, "profile-model");
        assert_eq!(
            resolved.connection,
            "rpi1:16:profile-provider13:profile-model"
        );

        let session_override = SessionExecutionSelection {
            provider_id: Some("fallback-provider".to_owned()),
            model_id: Some("fallback-model".to_owned()),
            ..SessionExecutionSelection::default()
        };
        let overridden = resolve_selected_runtime_model_with_profile(
            &models,
            &session_override,
            Some(AgentProfileDescriptor {
                reference: scriberig_app::AgentProfileRevisionRef {
                    identity: "profile-test".to_owned(),
                    revision: "revision-test".to_owned(),
                },
                name: "Profile test".to_owned(),
                components: Vec::new(),
                preferred_execution: Some(AgentProfileExecutionPreference {
                    provider_id: "profile-provider".to_owned(),
                    model_id: "profile-model".to_owned(),
                    thinking_level: None,
                }),
            }),
            "fallback-model",
        )
        .expect("session override resolves");

        assert_eq!(overridden.model.provider, "fallback-provider");
        assert_eq!(overridden.model.id, "fallback-model");
    }
}
