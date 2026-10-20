//! Verifies local provider profiles remain provenance outside provider requests.

use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelRequest, ModelResponse, ProviderMetadata,
};
use scriberig_agent_runtime::{AgentMessage, ModelProvider};
use scriberig_session::ModelRequestProjection;

/// Keeps the `ScribeRig` profile distinct from upstream provider identity.
#[test]
fn provider_profile_is_persistable_provenance_not_request_payload() {
    let (profile_a, request_a) = execute_with_profile("cli-profile-a");
    let (profile_b, _) = execute_with_profile("cli-profile-b");

    assert_eq!(profile_a.provider_id, None);
    assert_eq!(profile_a.provider_profile.as_deref(), Some("cli-profile-a"));
    assert_ne!(profile_a.request_fingerprint, profile_b.request_fingerprint);
    assert_eq!(request_a.metadata.get("provider_profile"), None);
    assert!(
        request_a
            .messages
            .iter()
            .all(|message| !message.content.contains("cli-profile-a"))
    );
}

/// Executes one fake provider request with a local profile identifier.
///
/// - `provider_profile`: Safe `ScribeRig` provider profile.
///
/// The captured provenance and normalized request received by the provider.
/// # Returns
/// The captured request projection and fake provider request.
fn execute_with_profile(provider_profile: &str) -> (ModelRequestProjection, ModelRequest) {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("completed".to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let mut model = ProviderModel::new(provider.clone(), "model-a")
        .with_optional_provider_profile(Some(provider_profile.to_owned()));

    model
        .decide(&[AgentMessage::User("safe prompt".to_owned())])
        .expect("provider response");
    let projection = model
        .take_request_projections()
        .pop()
        .expect("captured request projection");
    let request = provider.requests().pop().expect("provider request");
    (projection, request)
}
