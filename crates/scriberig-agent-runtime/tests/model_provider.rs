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

/// Verifies a configured thinking id is copied into each normalized request.
#[test]
fn provider_model_snapshots_the_selected_thinking_level() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("final".to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let mut model = ProviderModel::new(provider.clone(), "model")
        .with_thinking_level(Some("deep-dive".to_owned()));

    model
        .decide(&[AgentMessage::User("prompt".to_owned())])
        .expect("model decision");

    assert_eq!(
        provider.requests()[0].thinking_level.as_deref(),
        Some("deep-dive")
    );
    assert_eq!(provider.requests()[0].model, "model");
}

/// Captures bounded provider-neutral requests and clears them on consumption.
#[test]
fn provider_model_captures_a_bounded_safe_request_projection() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("final".to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let mut model =
        ProviderModel::new(provider, "model-a").with_thinking_level(Some("careful".to_owned()));

    model
        .decide(&[AgentMessage::User("x".repeat(20_000))])
        .expect("provider decision");
    let captured = model.take_request_projections();

    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].model, "model-a");
    assert_eq!(captured[0].thinking_level.as_deref(), Some("careful"));
    assert_eq!(captured[0].messages[0].content.chars().count(), 15_000);
    assert!(captured[0].truncated);
    assert_eq!(model.take_request_projections(), Vec::new());
}

/// Keeps execution arguments intact while deriving redacted display arguments.
#[test]
fn named_tool_arguments_follow_sensitive_schema_annotations() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: None,
        tool_calls: vec![ToolCall {
            id: "call-search".to_owned(),
            name: "search".to_owned(),
            arguments: serde_json::json!({
                "query": "architecture",
                "token": "FAKE_SECRET_MARKER_8391"
            }),
        }],
        finish_reason: FinishReason::ToolCall,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let tool = ToolDefinition {
        name: "search".to_owned(),
        description: "Search semantic content".to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "token": {"type": "string", "writeOnly": true}
            }
        }),
    };
    let catalog = ToolCatalog::new(vec![ToolDescriptor {
        definition: tool,
        result_shape: "bounded search result".to_owned(),
        invocation_label: Some("Search semantic content".to_owned()),
        state_changing: false,
        execution_mode: ToolExecutionMode::Atomic,
        required_capability: None,
    }]);
    let mut model = ProviderModel::new(provider, "model").with_catalog(catalog);

    let decision = model
        .decide(&[AgentMessage::User("search".to_owned())])
        .expect("named tool decision");

    let ModelDecision::NamedTool {
        arguments,
        presentation_arguments,
        presentation_label,
        ..
    } = decision
    else {
        panic!("expected a named tool decision");
    };
    assert_eq!(arguments["token"], "FAKE_SECRET_MARKER_8391");
    assert_eq!(presentation_arguments["token"], "[REDACTED]");
    assert_eq!(presentation_arguments["query"], "architecture");
    assert_eq!(
        presentation_label.as_deref(),
        Some("Search semantic content")
    );
    assert!(
        !presentation_arguments
            .to_string()
            .contains("FAKE_SECRET_MARKER_8391")
    );
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
        invocation_label: None,
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

/// Verifies parent requests inherit compact child results without child transcripts.
#[test]
fn child_result_is_compact_in_parent_provider_context() {
    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("continue".to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let mut model = ProviderModel::new(provider.clone(), "model");
    model
        .decide(&[
            AgentMessage::User("continue from architecture result".to_owned()),
            AgentMessage::Child(scriberig_agent_runtime::ChildResult {
                summary: "Use the shared code domain adapter pattern".to_owned(),
                handles: vec![
                    "project:prj-one".to_owned(),
                    "snapshot:snp-base".to_owned(),
                    "entity:ent-python-adapter".to_owned(),
                ],
                branch: None,
            }),
        ])
        .expect("parent model decision");

    let request = provider.requests().pop().expect("captured parent request");
    let context = request
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(context.contains("Use the shared code domain adapter pattern"));
    assert!(context.contains("project:prj-one"));
    assert!(context.contains("snapshot:snp-base"));
    assert!(!context.contains("child raw tool transcript sentinel"));
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
