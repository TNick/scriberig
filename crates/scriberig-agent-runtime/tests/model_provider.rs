//! Verifies normalized providers drive the existing runtime decision loop.

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderMetadata, ToolCall, ToolDefinition,
};
use scriberig_agent_runtime::tools::{ToolCatalog, ToolDescriptor, ToolExecutionMode};
use scriberig_agent_runtime::{AgentMessage, ModelDecision, ModelProvider};

/// Verifies a normalized final response becomes a runtime final decision.
#[test]
fn normalized_final_response_drives_runtime_decision() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("final".to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let mut model = ProviderModel::new(provider, "model");

    let decision = model
        .decide(&[AgentMessage::User("prompt".to_owned())])
        .expect("model decision");

    assert_eq!(decision, ModelDecision::Final("final".to_owned()));
}

/// Verifies a trusted delegate call becomes a compact child decision.
#[test]
fn normalized_delegate_response_drives_child_decision() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: None,
        tool_calls: vec![ToolCall {
            id: "call-child".to_owned(),
            name: "delegate".to_owned(),
            arguments: serde_json::json!({
                "purpose": "inspect architecture",
                "prompt": "find the relevant boundary"
            }),
        }],
        finish_reason: FinishReason::ToolCall,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let catalog = ToolCatalog::new(vec![ToolDescriptor {
        definition: ToolDefinition {
            name: "delegate".to_owned(),
            description: "Delegate focused work".to_owned(),
            parameters: serde_json::json!({"type": "object"}),
        },
        result_shape: "compact child result".to_owned(),
        state_changing: false,
        execution_mode: ToolExecutionMode::ChildBranch,
        required_capability: None,
    }]);
    let mut model = ProviderModel::new(provider, "model").with_catalog(catalog);

    let decision = model
        .decide(&[AgentMessage::User("delegate".to_owned())])
        .expect("child decision");

    assert_eq!(
        decision,
        ModelDecision::Child(scriberig_agent_runtime::ChildRequest {
            purpose: "inspect architecture".to_owned(),
            prompt: "find the relevant boundary".to_owned(),
        })
    );
}

/// Verifies exact model request reuse avoids a second provider invocation.
#[test]
fn provider_model_reuses_exact_cached_response() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("cached".to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let cache = ModelCache::new();
    let mut model =
        ProviderModel::new(provider, "model").with_cache(cache, CachePolicy::ReuseExact);
    let messages = [AgentMessage::User("same".to_owned())];

    assert_eq!(
        model.decide(&messages),
        Ok(ModelDecision::Final("cached".to_owned()))
    );
    assert_eq!(
        model.decide(&messages),
        Ok(ModelDecision::Final("cached".to_owned()))
    );
}
