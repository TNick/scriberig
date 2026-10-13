//! Verifies normalized providers drive the existing runtime decision loop.

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::context::{
    BranchExecutionContext, ContextCompiler, ContextPolicy, ProjectSemanticScope,
};
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
                "prompt": "find the relevant boundary",
                "capabilities": ["inspect.entity"],
                "expected_result": "compact architecture map"
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
            capabilities: vec!["inspect.entity".to_owned()],
            expected_result: "compact architecture map".to_owned(),
        })
    );
}

/// Verifies compiled exact project scope enters the normalized provider request.
#[test]
fn compiled_project_scope_reaches_provider_request() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("ready".to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let context = ContextCompiler::new(ContextPolicy::default()).compile(
        &BranchExecutionContext::new("session", "branch", "discover", "inspect code")
            .with_project_scope(ProjectSemanticScope {
                project: "prj-one".to_owned(),
                snapshot: "snp-base".to_owned(),
                root: "ent-root".to_owned(),
                selection: Some("ent-selected".to_owned()),
            }),
    );
    let mut model = ProviderModel::new(provider.clone(), "model").with_compiled_context(context);
    model
        .decide(&[AgentMessage::User("inspect code".to_owned())])
        .expect("model decision");

    let request = provider.requests().pop().expect("provider request");
    assert!(request.messages[0].content.contains("prj-one"));
    assert!(request.messages[0].content.contains("snp-base"));
    assert!(request.messages[0].content.contains("ent-root"));
    assert!(request.messages[0].content.contains("ent-selected"));
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
