//! Selects and delegates provider adapters used by the native TUI.

use scriberig_agent_runtime::provider::{
    ModelRequest, ModelResponse, ProviderAdapter, ProviderError,
};

/// Native provider choice across configured remote and deterministic scenarios.
#[derive(Clone)]
pub(super) enum NativeProvider {
    /// Remote OpenAI-compatible model endpoint.
    OpenAiCompatible(scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider),
    /// Deterministic onboarding scenario used by product tests.
    Deterministic(crate::deterministic_provider::BootstrapProvider),
    /// Deterministic failure after persisted manifest review.
    BootstrapImportFailure(crate::bootstrap_failure_provider::BootstrapImportFailureProvider),
    /// Deterministic PHP self-extension flow selected by product-gate configuration.
    SelfHostingPhp(crate::self_hosting_provider::PhpSelfHostingProvider),
}

impl ProviderAdapter for NativeProvider {
    /// Completes through the selected remote or deterministic adapter.
    ///
    /// # Parameters
    /// - `self`: Selected native provider.
    /// - `request`: Normalized provider request.
    ///
    /// # Returns
    /// The normalized provider response.
    ///
    /// # Errors
    /// Returns a classified provider failure.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        match self {
            Self::OpenAiCompatible(provider) => provider.complete(request),
            Self::Deterministic(provider) => provider.complete(request),
            Self::BootstrapImportFailure(provider) => provider.complete(request),
            Self::SelfHostingPhp(provider) => provider.complete(request),
        }
    }
}
