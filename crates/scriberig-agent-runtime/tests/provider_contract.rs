//! Verifies the provider-neutral model contract and deterministic fake.

use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelMessage, ModelRequest, ModelResponse, ProviderAdapter,
    ProviderMetadata, ToolCall,
};

/// Verifies a fake response is returned without a network dependency.
#[test]
fn fake_provider_returns_scripted_response_and_records_request() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("done".to_owned()),
        tool_calls: vec![ToolCall {
            id: "call-1".to_owned(),
            name: "inspect".to_owned(),
            arguments: serde_json::json!({"entity": "ent_1"}),
        }],
        finish_reason: FinishReason::ToolCall,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let request = ModelRequest::new("test-model", vec![ModelMessage::user("hello")]);

    let response = provider.complete(&request).expect("fake response");

    assert_eq!(response.text.as_deref(), Some("done"));
    assert_eq!(provider.requests(), vec![request]);
}
