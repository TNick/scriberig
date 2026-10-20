//! Verifies model request history survives a failed provider call.

use scriberig_agent_runtime::{
    AgentMessage, AgentRuntime, ModelDecision, ModelProvider, UnconfiguredTool,
};
use scriberig_app::ApplicationContext;
use scriberig_session::{ModelRequestMessage, ModelRequestProjection};

/// Returns one captured request and then fails its provider decision.
struct FailingProvider {
    /// Safe projection queued before the simulated transport failure.
    requests: Vec<ModelRequestProjection>,
}

impl ModelProvider for FailingProvider {
    /// Simulates a provider failure after its request was prepared.
    ///
    /// - `_messages`: Provider-neutral conversation ignored by this fixture.
    ///
    /// No decision because the simulated transport fails.
    /// # Returns
    /// Always returns the fixture error.
    /// # Errors
    /// Returns a provider failure after request preparation.
    fn decide(&mut self, _messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        Err("fixture transport failure".to_owned())
    }

    /// Returns and clears the prepared request projection.
    ///
    /// The request submitted before the fixture failure.
    /// # Returns
    /// The queued request projection.
    fn take_request_projections(&mut self) -> Vec<ModelRequestProjection> {
        std::mem::take(&mut self.requests)
    }
}

/// Retains a request projection when the provider fails after preparation.
#[test]
fn provider_failure_keeps_bounded_request_history_available() {
    let request = ModelRequestProjection {
        ordinal: 0,
        model: "model-a".to_owned(),
        provider_id: Some("provider-a".to_owned()),
        provider_profile: None,
        thinking_level: Some("careful".to_owned()),
        profile: None,
        active_components: Vec::new(),
        context_fingerprint: None,
        request_fingerprint: None,
        messages: vec![ModelRequestMessage {
            role: "user".to_owned(),
            content: "bounded prompt".to_owned(),
        }],
        tools: Vec::new(),
        truncated: false,
    };
    let mut runtime = AgentRuntime::new(
        FailingProvider {
            requests: vec![request],
        },
        UnconfiguredTool,
        1,
    );

    let error = runtime
        .run(
            &ApplicationContext::empty(),
            "prompt",
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect_err("provider failure");

    assert_eq!(
        error.to_string(),
        "agent provider: fixture transport failure"
    );
    let requests = runtime.take_captured_model_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].provider_id.as_deref(), Some("provider-a"));
    assert_eq!(requests[0].model, "model-a");
    assert_eq!(requests[0].messages[0].content, "bounded prompt");
}
