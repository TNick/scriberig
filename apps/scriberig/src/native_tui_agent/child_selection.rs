//! Resolves explicit delegated child assignments against one upstream snapshot.

use scriberig_agent_runtime::{ChildProviderBinding, ChildProviderResolver};
use scriberig_app::{ApplicationBackend, ApplicationService};
use scriberig_session::{ModelAssignment, SessionExecutionSelection};

use super::NativeTuiAgentRunner;

impl NativeTuiAgentRunner {
    /// Builds a resolver for explicit child provider/model assignments.
    ///
    /// The resolver captures the current upstream model snapshot and host
    /// provider configuration so descendants use the same catalog generation
    /// as the root invocation.
    ///
    /// # Parameters
    /// - `service`: Application service supplying the trusted upstream catalog.
    ///
    /// # Returns
    /// A host-authorized resolver for provider/model selections.
    pub(super) fn child_provider_resolver<B: ApplicationBackend>(
        &self,
        service: &ApplicationService<B>,
    ) -> ChildProviderResolver {
        let models = service.upstream_models();
        let providers = self.providers.clone();
        let extensions = self.provider_extension_runtime.clone();
        let credentials = self.upstream_credentials.clone();
        let upstream_enabled = self.upstream_enabled;
        let fallback_model = self.model.clone();
        std::sync::Arc::new(move |assignment: &ModelAssignment| {
            let selection = SessionExecutionSelection {
                provider_id: Some(assignment.provider_id.clone()),
                model_id: Some(assignment.model_id.clone()),
                thinking_level: assignment.thinking_level.clone(),
                ..SessionExecutionSelection::default()
            };
            let resolved = super::model_selection::resolve_selected_runtime_model_with_model(
                &models,
                &selection,
                &fallback_model,
            )
            .map_err(|_| "child provider/model assignment is unavailable".to_owned())?;
            let binding = super::provider_config::binding::provider_for(
                &providers,
                &extensions,
                credentials.as_ref(),
                upstream_enabled,
                &resolved.model,
                &resolved.connection,
            )
            .map_err(|_| "child provider/model assignment is unavailable".to_owned())?
            .ok_or_else(|| "child provider/model assignment is unavailable".to_owned())?;
            let (provider, model, request_secret) = binding;
            Ok(ChildProviderBinding {
                provider,
                model,
                request_secret,
                bearer: true,
                thinking_level: resolved.thinking_level.unwrap_or_default(),
                provider_profile: Some(resolved.connection),
            })
        })
    }
}
