//! Provides a deterministic post-manifest onboarding failure scenario.

use scriberig_agent_runtime::provider::{
    ModelRequest, ModelResponse, ProviderAdapter, ProviderError,
};

use crate::deterministic_provider::BootstrapProvider;

/// Fails onboarding after manifest review to exercise shared import rejection.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct BootstrapImportFailureProvider;

impl ProviderAdapter for BootstrapImportFailureProvider {
    /// Runs normal onboarding tools with one invalid reviewed-manifest reference.
    ///
    /// # Parameters
    /// - `self`: Deterministic failure scenario.
    /// - `request`: Normalized model request from the agent runtime.
    ///
    /// # Returns
    /// Normal bootstrap response until shared import validation rejects the reference.
    ///
    /// # Errors
    /// Returns the bootstrap provider's structured failure.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        let mut response = BootstrapProvider.complete(request)?;
        if let Some(import) = response
            .tool_calls
            .iter_mut()
            .find(|call| call.name == "project_import")
        {
            import.arguments["manifest_id"] = serde_json::Value::String("0".repeat(64));
        }
        Ok(response)
    }
}
