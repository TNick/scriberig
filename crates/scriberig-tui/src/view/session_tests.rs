//! Verifies the semantic session tree projection.

use super::{SessionModel, SessionRowKind};
use scriberig_core::EntityId;
use scriberig_session::{Branch, BranchStatus, Session, Turn};

/// Verifies branches and turns retain their semantic nesting and identity.
#[test]
fn builds_nested_session_rows() {
    let branch_id = EntityId::from_bytes([2; 16]);
    let mut branch = Branch::new(branch_id, "exploration").expect("branch");
    branch.push_turn(Turn {
        id: EntityId::from_bytes([3; 16]),
        ordinal: 0,
        human: None,
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    let session = Session {
        id: EntityId::from_bytes([1; 16]),
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let persisted = serde_json::to_vec(&session).expect("persist session snapshot");
    let restored: Session = serde_json::from_slice(&persisted).expect("restore session snapshot");
    let model = SessionModel::from_session(&restored);
    assert_eq!(model.rows[0].kind, SessionRowKind::Session);
    assert_eq!(model.rows[1].depth, 1);
    assert_eq!(model.rows[2].depth, 2);
    assert_eq!(model.reference_at(2), Some(EntityId::from_bytes([3; 16])));
    assert!(model.rows[1].transient);
}

/// Projects a very deep branch chain without recursive traversal or quadratic scans.
#[test]
fn projects_deep_branch_chains_iteratively() {
    let depth = 12_000_usize;
    let identity = |value: u64| {
        let mut bytes = [0_u8; 16];
        bytes[8..].copy_from_slice(&value.to_be_bytes());
        EntityId::from_bytes(bytes)
    };
    let root_id = identity(1);
    let mut branches = vec![Branch::new(root_id, "root").expect("root branch")];
    let mut parent = root_id;
    for index in 0..depth {
        let id = identity(u64::try_from(index + 2).expect("bounded branch identity"));
        branches.push(
            Branch::new_child(id, parent, format!("child {index}"), "deep chain")
                .expect("child branch"),
        );
        parent = id;
    }
    let session = Session {
        id: identity(0),
        branches,
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let model = SessionModel::from_session(&session);

    assert_eq!(model.rows.len(), depth + 2);
    assert_eq!(model.rows[1].reference, root_id);
    assert_eq!(model.rows[1].depth, 1);
    assert_eq!(
        model.rows.last().expect("deepest branch row").depth,
        depth + 1
    );
}

/// Verifies durable assistant output remains visible in the session projection.
#[test]
fn shows_durable_agent_output_in_turn_row() {
    let branch_id = EntityId::from_bytes([22; 16]);
    let mut branch = Branch::new(branch_id, "agent root").expect("branch");
    branch.push_turn(Turn {
        id: EntityId::from_bytes([23; 16]),
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "Onboard the current project.".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "The current project is onboarded and ready.".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    let session = Session {
        id: EntityId::from_bytes([21; 16]),
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let persisted = serde_json::to_vec(&session).expect("persist session snapshot");
    let restored: Session = serde_json::from_slice(&persisted).expect("restore session snapshot");
    let model = SessionModel::from_session(&restored);

    assert!(model.rows.iter().any(|row| {
        row.kind == SessionRowKind::Turn
            && row
                .label
                .contains("The current project is onboarded and ready.")
    }));
}

/// Verifies a shared selection update refreshes the visible session header.
#[test]
fn updated_execution_selection_refreshes_session_header() {
    let session = Session {
        id: EntityId::from_bytes([33; 16]),
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut model = SessionModel::from_session(&session);

    model.set_execution(scriberig_session::SessionExecutionSelection {
        revision: 2,
        provider_id: Some("provider-a".to_owned()),
        model_id: Some("model-a".to_owned()),
        thinking_level: Some("careful".to_owned()),
        history: Vec::new(),
    });

    assert_eq!(model.execution.revision, 2);
    assert!(model.rows[0].label.contains("Model model-a"));
    assert!(model.rows[0].label.contains("Thinking careful"));
}

/// Verifies model and tool records render as nested rows without raw payloads.
#[test]
fn renders_durable_model_and_tool_activity_without_raw_tool_output() {
    use scriberig_session::{
        CacheDisposition, ExecutionActivity, ModelInvocation, ModelOutput, ToolInvocation,
        ToolResult,
    };

    let session_id = EntityId::from_bytes([51; 16]);
    let branch_id = EntityId::from_bytes([52; 16]);
    let invocation_id = EntityId::from_bytes([53; 16]);
    let tool_id = EntityId::from_bytes([54; 16]);
    let result_id = EntityId::from_bytes([55; 16]);
    let output_id = EntityId::from_bytes([56; 16]);
    let mut branch = Branch::new(branch_id, "main").expect("branch");
    branch.activity = vec![
        ExecutionActivity::ModelInvocation(invocation_id),
        ExecutionActivity::ToolInvocation(tool_id),
        ExecutionActivity::ToolResult(result_id),
        ExecutionActivity::ModelOutput(output_id),
    ];
    branch.model_invocations.push(ModelInvocation {
        id: invocation_id,
        branch: branch_id,
        operation: None,
        ordinal: 0,
        agent_profile: None,
        context_fingerprint: None,
        provider_profile: None,
        provider_id: None,
        model: Some("model-a".to_owned()),
        model_profile: None,
        provider_model_id: None,
        thinking_level: Some("careful".to_owned()),
        request_fingerprint: None,
        cache: CacheDisposition::NotUsed,
        output: Some(output_id),
        request: None,
    });
    branch.tool_invocations.push(ToolInvocation {
        id: tool_id,
        name: "inspect".to_owned(),
        input: br#"{"private_payload":"SECRET_MARKER"}"#.to_vec(),
        parent_model_invocation: Some(invocation_id),
        presentation_input: Some(
            br#"{"filter":{"path":"src/lib.rs"},"token":"[REDACTED]"}"#.to_vec(),
        ),
        presentation_label: None,
        category: Some(scriberig_session::ToolActivityKind::SemanticMutation),
    });
    branch.tool_results.push(ToolResult {
        id: result_id,
        invocation: tool_id,
        output: br#"{"private_payload":"SECRET_MARKER"}"#.to_vec(),
        failed: false,
        presentation_preview: Some(
            br#"{"summary":"Three matches","private_payload":"SECRET_MARKER"}"#.to_vec(),
        ),
    });
    branch.model_outputs.push(ModelOutput {
        id: output_id,
        invocation: invocation_id,
        text: Some("Completed safely.".to_owned()),
        structured_output: None,
        finish_reason: None,
        usage: None,
    });
    let session = Session {
        id: session_id,
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let model = SessionModel::from_session(&session);
    assert!(model.rows.iter().any(|row| {
        row.kind == SessionRowKind::ToolInvocation && row.label.starts_with("Semantic operation:")
    }));
    assert_durable_activity_is_safe(&model, invocation_id, tool_id);

    let mut process_session = session;
    process_session.branches[0].tool_invocations[0].category =
        Some(scriberig_session::ToolActivityKind::OpaqueFallback);
    let fallback_model = SessionModel::from_session(&process_session);
    assert!(fallback_model.rows.iter().any(|row| {
        row.kind == SessionRowKind::ToolInvocation && row.label.starts_with("Fallback tool:")
    }));

    process_session.branches[0].tool_invocations[0].category =
        Some(scriberig_session::ToolActivityKind::ValidationProcess);
    let process_model = SessionModel::from_session(&process_session);
    assert!(process_model.rows.iter().any(|row| {
        row.kind == SessionRowKind::ToolInvocation && row.label.starts_with("Run validation:")
    }));
}

/// Verifies projected activity details stay sanitized before presentation.
///
/// - `model`: Projection built from durable execution activity.
/// - `invocation_id`: Stable model invocation identity.
/// - `tool_id`: Stable tool invocation identity.
fn assert_durable_activity_is_safe(
    model: &SessionModel,
    invocation_id: EntityId,
    tool_id: EntityId,
) {
    let invocation = model
        .rows
        .iter()
        .find(|row| row.kind == SessionRowKind::ModelInvocation)
        .expect("model invocation row");
    let tool = model
        .rows
        .iter()
        .find(|row| row.kind == SessionRowKind::ToolInvocation)
        .expect("tool invocation row");
    let result = model
        .rows
        .iter()
        .find(|row| row.kind == SessionRowKind::ToolResult)
        .expect("tool result row");

    assert!(invocation.label.contains("model-a"));
    assert!(invocation.label.contains("careful"));
    assert!(tool.label.contains("inspect"));
    assert_eq!(
        model
            .tool_invocation_details
            .get(&tool_id)
            .map(String::as_str),
        Some(
            "{\n  \"filter\": {\n    \"path\": \"src/lib.rs\"\n  },\n  \"token\": \"[REDACTED]\"\n}",
        )
    );
    assert!(result.depth > tool.depth);
    assert!(result.label.contains("Three matches"));
    assert!(
        !model
            .rows
            .iter()
            .any(|row| row.label.contains("SECRET_MARKER"))
    );
    assert!(
        !model
            .tool_invocation_details
            .values()
            .any(|detail| detail.contains("SECRET_MARKER"))
    );
    assert!(model.tool_invocation_details[&tool_id].contains("[REDACTED]"));

    assert!(model.rows.iter().any(|row| row.reference == invocation_id));
    assert!(model.tool_invocation_details[&tool_id].contains("[REDACTED]"));
    assert!(!model.tool_invocation_details[&tool_id].contains("SECRET_MARKER"));
    assert!(!model.tool_invocation_details[&tool_id].contains("private_payload"));
}

/// Shows current session model and thinking selection in the session header row.
#[test]
fn shows_shared_model_and_thinking_selection() {
    let session = Session {
        id: EntityId::from_bytes([24; 16]),
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection {
            revision: 4,
            provider_id: Some("provider-b".to_owned()),
            model_id: Some("model-b".to_owned()),
            thinking_level: Some("deliberate".to_owned()),
            history: Vec::new(),
        },
    };

    let model = SessionModel::from_session(&session);

    assert!(model.rows[0].label.contains("Model model-b"));
    assert!(model.rows[0].label.contains("Thinking deliberate"));
}

/// Verifies completed child branches collapse to result references.
#[test]
fn collapses_completed_child_activity() {
    let parent_id = EntityId::from_bytes([8; 16]);
    let child_id = EntityId::from_bytes([9; 16]);
    let turn_id = EntityId::from_bytes([10; 16]);
    let result_id = EntityId::from_bytes([11; 16]);
    let parent = Branch::new(parent_id, "parent").expect("parent");
    let mut child = Branch::new(child_id, "child").expect("child");
    child.parent = Some(parent_id);
    child.turns.push(Turn {
        id: turn_id,
        ordinal: 0,
        human: None,
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    child.status = BranchStatus::Completed;
    child.result = Some(scriberig_session::BranchResult {
        id: result_id,
        branch: child_id,
        status: BranchStatus::Completed,
        summary: Some("done".to_owned()),
        result_kind: Some("agent".to_owned()),
        candidate_context: None,
        references: vec![result_id],
        result_handles: vec![
            "candidate:pc_123".to_owned(),
            format!("snapshot:{}", scriberig_core::SnapshotId::new([13; 32])),
        ],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    let session = Session {
        id: EntityId::from_bytes([12; 16]),
        branches: vec![parent, child],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let persisted = serde_json::to_vec(&session).expect("persist session snapshot");
    let restored: Session = serde_json::from_slice(&persisted).expect("restore session snapshot");
    let model = SessionModel::from_session(&restored);
    assert!(model.rows.iter().any(|row| row.reference == result_id));
    assert!(!model.rows.iter().any(|row| row.reference == turn_id));
    let review = model
        .rows
        .iter()
        .find(|row| row.kind == SessionRowKind::Candidate)
        .expect("candidate review row");
    assert_eq!(review.candidate.as_deref(), Some("pc_123"));
    assert_eq!(
        review.snapshot,
        Some(scriberig_core::SnapshotId::new([13; 32]))
    );
}
