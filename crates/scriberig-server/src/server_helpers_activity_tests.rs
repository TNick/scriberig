//! Verifies safe server activity projections.

use super::branch_execution_activity;
use scriberig_core::{EntityId, OperationId};
use scriberig_session::{
    AgentOutput, Branch, CacheDisposition, ExecutionActivity, HumanInput, ModelInvocation,
    ToolInvocation, ToolResult, Turn,
};

/// Projects tool links without serializing argument or result bytes.
#[test]
fn activity_projection_redacts_tool_payload_bytes() {
    let mut branch = Branch::new(EntityId::new(), "root").expect("branch");
    let model_id = EntityId::new();
    let invocation_id = EntityId::new();
    let result_id = EntityId::new();
    let child_id = EntityId::new();
    branch.model_invocations.push(ModelInvocation {
        id: model_id,
        branch: branch.id,
        operation: Some(OperationId::new("agent-run").expect("operation id")),
        ordinal: 0,
        agent_profile: None,
        context_fingerprint: None,
        provider_profile: None,
        provider_id: None,
        model: Some("model-a".to_owned()),
        model_profile: None,
        provider_model_id: None,
        thinking_level: None,
        request_fingerprint: None,
        cache: CacheDisposition::NotUsed,
        output: None,
        request: None,
    });
    branch.tool_invocations.push(ToolInvocation {
        id: invocation_id,
        name: "search".to_owned(),
        input: b"fake-sensitive-tool-input".to_vec(),
        parent_model_invocation: Some(model_id),
        presentation_input: Some(br#"{"query":"find documents","token":"[REDACTED]"}"#.to_vec()),
        presentation_label: Some("Find documents".to_owned()),
        category: Some(scriberig_session::ToolActivityKind::SemanticRead),
    });
    branch.tool_results.push(ToolResult {
        id: result_id,
        invocation: invocation_id,
        output: b"fake-sensitive-tool-output".to_vec(),
        failed: false,
        presentation_preview: None,
    });
    branch.activity.extend([
        ExecutionActivity::ModelInvocation(model_id),
        ExecutionActivity::ToolInvocation(invocation_id),
        ExecutionActivity::ToolResult(result_id),
        ExecutionActivity::ChildBranchSpawned {
            branch: child_id,
            parent_tool_invocation: invocation_id,
        },
    ]);

    let projected = branch_execution_activity("session-1", &branch);
    let encoded = serde_json::to_string(&projected).expect("serialize activity");
    let invocation_reference = invocation_id.to_string();
    let model_reference = model_id.to_string();

    assert_eq!(projected.len(), 4);
    assert_eq!(projected[0].operation.as_deref(), Some("op_agent-run"));
    assert_eq!(projected[1].summary, "Find documents");
    assert_eq!(
        projected[1].durable_ref.as_deref(),
        Some(invocation_reference.as_str())
    );
    assert_eq!(
        projected[1].tool_arguments.as_ref().unwrap()["token"],
        "[REDACTED]"
    );
    assert_eq!(
        projected[1].parent.as_deref(),
        Some(model_reference.as_str())
    );
    assert_eq!(
        projected[2].parent.as_deref(),
        Some(invocation_reference.as_str())
    );
    assert_eq!(
        projected[3].durable_ref.as_deref(),
        Some(child_id.to_string().as_str())
    );
    assert_eq!(
        projected[3].parent.as_deref(),
        Some(invocation_reference.as_str())
    );
    assert!(!encoded.contains("fake-sensitive-tool-input"));
    assert!(!encoded.contains("fake-sensitive-tool-output"));
    assert!(encoded.contains("search"));
}

/// Projects human input and committed output without exposing prompt text.
#[test]
fn activity_projection_includes_turn_lifecycle_safely() {
    let mut branch = Branch::new(EntityId::new(), "root").expect("branch");
    let turn_id = EntityId::new();
    branch.push_turn(Turn {
        id: turn_id,
        ordinal: 0,
        human: Some(HumanInput {
            text: "fake-prompt-secret".to_owned(),
        }),
        agent: Some(AgentOutput {
            text: "safe final answer".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    branch.activity.extend([
        ExecutionActivity::HumanInput(turn_id),
        ExecutionActivity::AgentOutput(turn_id),
    ]);

    let projected = branch_execution_activity("session-1", &branch);
    let encoded = serde_json::to_string(&projected).expect("serialize activity");

    assert_eq!(projected[0].summary, "Human input");
    assert_eq!(projected[1].summary, "safe final answer");
    assert!(!encoded.contains("fake-prompt-secret"));
    assert_eq!(projected[0].session, "session-1");
}

/// Projects only the tool boundary summary and bounds its display length.
#[test]
fn activity_projection_uses_safe_bounded_tool_result_summary() {
    let mut branch = Branch::new(EntityId::new(), "root").expect("branch");
    let invocation_id = EntityId::new();
    let result_id = EntityId::new();
    branch.tool_invocations.push(ToolInvocation {
        id: invocation_id,
        name: "search".to_owned(),
        input: Vec::new(),
        parent_model_invocation: None,
        presentation_input: None,
        presentation_label: None,
        category: Some(scriberig_session::ToolActivityKind::SemanticRead),
    });
    branch.tool_results.push(ToolResult {
        id: result_id,
        invocation: invocation_id,
        output: serde_json::to_vec(&serde_json::json!({
            "summary": "s".repeat(600),
            "private_payload": "never exposed",
        }))
        .expect("result JSON"),
        failed: false,
        presentation_preview: None,
    });
    branch
        .activity
        .push(ExecutionActivity::ToolResult(result_id));

    let projected = branch_execution_activity("session-1", &branch);
    let encoded = serde_json::to_string(&projected).expect("activity JSON");

    assert_eq!(projected[0].summary.chars().count(), 513);
    assert!(projected[0].summary.ends_with('…'));
    assert!(!encoded.contains("private_payload"));
    assert!(!encoded.contains("never exposed"));
    assert_eq!(
        projected[0].tool_result_preview.as_ref().unwrap()["summary"],
        "s".repeat(512)
    );
    assert!(
        projected[0]
            .tool_result_preview
            .as_ref()
            .unwrap()
            .get("private_payload")
            .is_none()
    );
}
