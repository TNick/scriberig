//! Verifies typed upstream thinking values stay scalar in protocol rows.

use super::*;
use scriberig_app::{
    ApplicationError, ApplicationRequest, ApplicationResult, CapabilityContext, CapabilityId,
};
use std::collections::BTreeMap;

/// Backend stub for model-library projection tests.
struct ProjectionBackend;

impl ApplicationBackend for ProjectionBackend {
    /// Rejects semantic operations because these tests only read model metadata.
    ///
    /// - `self`: Backend stub.
    /// - `_context`: Unused request context.
    /// - `_request`: Unsupported semantic request.
    ///
    /// No semantic result is produced.
    ///
    /// Returns an unsupported-operation error for every request.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model projection fixture".to_owned(),
        })
    }
}

/// Projects upstream model-specific levels to ordered scalar protocol IDs.
#[test]
fn model_library_preserves_upstream_thinking_values_and_order() {
    let mut first = rpi_ai::Model::new(
        "wire-alpha",
        "Alpha",
        rpi_ai::Api::AnthropicMessages,
        "provider-alpha",
        "https://example.test",
    );
    first.thinking_level_map = Some(BTreeMap::from([
        (rpi_ai::ThinkingLevel::High, None),
        (rpi_ai::ThinkingLevel::Medium, Some("balanced".to_owned())),
        (rpi_ai::ThinkingLevel::Xhigh, Some("extended".to_owned())),
    ]));
    let second = rpi_ai::Model::new(
        "wire-beta",
        "Beta",
        rpi_ai::Api::AnthropicMessages,
        "provider-beta",
        "https://example.test",
    );
    let service = scriberig_app::ApplicationService::new(ProjectionBackend)
        .with_upstream_models(vec![first.clone(), second.clone()]);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_model_library").expect("model-library capability")
        ]),
        ..ApplicationContext::default()
    };

    let response = model_library(&service, &context).expect("model-library response");
    let alpha = response
        .models
        .iter()
        .find(|model| model.provider_model_id == first.id)
        .expect("alpha model row");
    let beta = response
        .models
        .iter()
        .find(|model| model.provider_model_id == second.id)
        .expect("beta model row");

    assert_eq!(
        alpha.thinking_levels,
        first
            .supported_thinking_levels()
            .iter()
            .map(|level| level.as_str().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        beta.thinking_levels,
        second
            .supported_thinking_levels()
            .iter()
            .map(|level| level.as_str().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        alpha.thinking_levels,
        vec!["off", "minimal", "low", "medium", "xhigh"]
    );
    assert_eq!(
        beta.thinking_levels,
        vec!["off", "minimal", "low", "medium", "high"]
    );
}
