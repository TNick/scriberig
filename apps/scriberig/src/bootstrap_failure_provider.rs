//! Scripts a deterministic post-manifest onboarding failure scenario.

use rpi_ai::{Context, Model};

use crate::deterministic_provider::BootstrapProvider;

/// Fails onboarding after manifest review to exercise shared import rejection.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct BootstrapImportFailureProvider;

impl BootstrapImportFailureProvider {
    /// Runs onboarding and corrupts the reviewed manifest reference.
    ///
    /// # Parameters
    /// - `context`: Exact upstream provider context.
    /// - `model`: Exact upstream model bound to the scenario.
    ///
    /// # Returns
    /// The bootstrap response, with an invalid manifest reference when importing.
    ///
    /// # Errors
    /// Returns the bootstrap scenario error when required tool evidence is absent.
    pub(super) fn respond(
        context: &Context,
        model: &Model,
    ) -> Result<rpi_ai::AssistantMessage, String> {
        let context_debug = format!("{context:?}");
        if context_debug.matches("tool error:").count() >= 3 {
            return Err("project import failed: manifest_changed".to_owned());
        }
        let mut response = BootstrapProvider::respond(context, model)?;
        for content in &mut response.content {
            let rpi_ai::types::Content::ToolCall(call) = content else {
                continue;
            };
            if call.name == "project_import" {
                call.arguments["manifest_id"] = serde_json::Value::String("0".repeat(64));
            }
        }
        Ok(response)
    }
}
