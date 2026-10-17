//! Verifies provider diagnostics use a bounded request and propagate failures.

use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderAdapter, ProviderError, ProviderMetadata,
};

/// Connection checks identify the model and allow brief reasoning.
#[test]
fn connection_check_uses_the_selected_model_and_bounded_request() {
    let provider =
        FakeProvider::from_responses([Ok(response(FinishReason::Stop, Some("MODEL OK")))]);
    provider
        .check_connection("configured-model")
        .expect("successful check");
    let request = &provider.requests()[0];
    assert_eq!(request.model, "configured-model");
    assert_eq!(request.max_tokens, Some(512));
    assert_eq!(request.tools, Vec::new());
    assert_eq!(request.messages.len(), 1);
    assert!(request.messages[0].content.contains("MODEL OK"));
    assert!(request.messages[0].content.contains("reasoning"));
}

/// Reasoning may precede the required final connection marker.
#[test]
fn connection_check_accepts_reasoning_before_final_marker() {
    let provider = FakeProvider::from_responses([Ok(response(
        FinishReason::Stop,
        Some("I have checked the request.\nMODEL OK\n"),
    ))]);
    assert!(provider.check_connection("configured-model").is_ok());
}

/// A response without the final marker is not proof of model availability.
#[test]
fn connection_check_requires_final_marker() {
    let provider = FakeProvider::from_responses([Ok(response(
        FinishReason::Stop,
        Some("The model received the request."),
    ))]);
    assert_eq!(
        provider.check_connection("configured-model"),
        Err(ProviderError::MalformedResponse(
            "provider did not finish the connection check with MODEL OK".to_owned()
        ))
    );
}

/// Connection checks retain provider authentication and transport categories.
#[test]
fn connection_check_propagates_provider_failures() {
    let provider = FakeProvider::from_responses([Err(ProviderError::Authentication(
        "provider authentication failed".to_owned(),
    ))]);
    assert_eq!(
        provider.check_connection("configured-model"),
        Err(ProviderError::Authentication(
            "provider authentication failed".to_owned()
        ))
    );
}

/// A refusal is not reported as a successful connection check.
#[test]
fn connection_check_rejects_provider_refusals() {
    let provider = FakeProvider::from_responses([Ok(response(FinishReason::Refusal, None))]);
    assert_eq!(
        provider.check_connection("configured-model"),
        Err(ProviderError::MalformedResponse(
            "provider refused the connection check".to_owned()
        ))
    );
}

/// Creates a minimal normalized fixture response.
///
/// # Parameters
/// - `finish_reason`: Terminal response behavior under test.
/// - `text`: Optional final text returned by the provider.
///
/// # Returns
/// A provider response with no generated content or external metadata.
fn response(finish_reason: FinishReason, text: Option<&str>) -> ModelResponse {
    ModelResponse {
        text: text.map(str::to_owned),
        tool_calls: Vec::new(),
        finish_reason,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}
