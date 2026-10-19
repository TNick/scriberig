//! Tests remote activity reconstruction.

use super::restore_activity;
use crate::view::session::SessionModel;
use scriberig_core::EntityId;
use scriberig_protocol::{SessionActivityItem, SessionActivityKind, SessionActivityState};
use scriberig_session::{Branch, Session};

/// Verifies remote hydration retains model, tool, result, and child causality.
#[test]
fn restored_tool_activity_keeps_model_parent_chain() {
    let model = EntityId::new();
    let tool = EntityId::new();
    let result = EntityId::new();
    let child = EntityId::new();
    let mut branch = Branch::new(EntityId::new(), "root").expect("branch");
    let branch_id = branch.id.to_string();
    restore_activity(
        &mut branch,
        vec![
            SessionActivityItem {
                id: model.to_string(),
                session: "session-1".to_owned(),
                sequence: 0,
                branch: branch_id.clone(),
                parent: None,
                kind: SessionActivityKind::ModelInvocation,
                state: SessionActivityState::Completed,
                summary: "Model request".to_owned(),
                durable_ref: Some(model.to_string()),
                model: Some("model-a".to_owned()),
                thinking_level: None,
                tool_name: None,
                tool_category: None,
                tool_arguments: None,
                tool_result_preview: None,
            },
            SessionActivityItem {
                id: tool.to_string(),
                session: "session-1".to_owned(),
                sequence: 1,
                branch: branch_id.clone(),
                parent: Some(model.to_string()),
                kind: SessionActivityKind::ToolInvocation,
                state: SessionActivityState::Completed,
                summary: "Search semantic content".to_owned(),
                durable_ref: Some(tool.to_string()),
                model: None,
                thinking_level: None,
                tool_name: Some("search".to_owned()),
                tool_category: Some(scriberig_protocol::ToolActivityKind::SemanticRead),
                tool_arguments: None,
                tool_result_preview: None,
            },
            SessionActivityItem {
                id: result.to_string(),
                session: "session-1".to_owned(),
                sequence: 2,
                branch: branch_id.clone(),
                parent: Some(tool.to_string()),
                kind: SessionActivityKind::ToolResult,
                state: SessionActivityState::Completed,
                summary: "Found one result".to_owned(),
                durable_ref: Some(result.to_string()),
                model: None,
                thinking_level: None,
                tool_name: None,
                tool_category: None,
                tool_arguments: None,
                tool_result_preview: None,
            },
            SessionActivityItem {
                id: child.to_string(),
                session: "session-1".to_owned(),
                sequence: 3,
                branch: branch_id,
                parent: Some(tool.to_string()),
                kind: SessionActivityKind::BranchStarted,
                state: SessionActivityState::Running,
                summary: "Child branch".to_owned(),
                durable_ref: Some(child.to_string()),
                model: None,
                thinking_level: None,
                tool_name: None,
                tool_category: None,
                tool_arguments: None,
                tool_result_preview: None,
            },
        ],
    )
    .expect("activity hydration");

    assert_eq!(
        branch.tool_invocations[0].parent_model_invocation,
        Some(model)
    );
    assert_eq!(
        branch.tool_invocations[0].presentation_label.as_deref(),
        Some("Search semantic content")
    );
    assert_eq!(branch.tool_results[0].invocation, tool);
    assert!(
        branch
            .activity
            .contains(&scriberig_session::ExecutionActivity::ChildBranchSpawned {
                branch: child,
                parent_tool_invocation: tool,
            })
    );
}

/// Restores the shared cross-surface redaction fixture without changing IDs.
#[test]
fn shared_fixture_preserves_causal_activity_identity() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/agent_console_sensitive_activity.json"
    ))
    .expect("cross-surface fixture");
    let items = serde_json::from_value(fixture["protocolActivity"].clone())
        .expect("protocol activity items");
    let branch_id = EntityId::from_bytes([0x52; 16]);
    let mut branch = Branch::new(branch_id, "root").expect("branch");

    restore_activity(&mut branch, items).expect("restore shared activity");

    assert_eq!(
        branch.model_invocations[0].id,
        EntityId::from_bytes([0x53; 16])
    );
    assert_eq!(
        branch.tool_invocations[0].id,
        EntityId::from_bytes([0x54; 16])
    );
    assert_eq!(branch.tool_results[0].id, EntityId::from_bytes([0x55; 16]));
    assert_eq!(
        branch.tool_invocations[0].parent_model_invocation,
        Some(branch.model_invocations[0].id)
    );
    assert_eq!(
        branch.tool_results[0].invocation,
        branch.tool_invocations[0].id
    );
    assert_eq!(
        branch.tool_invocations[0].presentation_input.as_deref(),
        Some(br#"{"query":"find documents","token":"[REDACTED]"}"#.as_slice())
    );
    let session_id = EntityId::from_bytes([0x51; 16]);
    let session_model = SessionModel::from_session(&Session {
        id: session_id,
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    });
    let activity_ids = session_model
        .rows
        .iter()
        .map(|row| row.reference)
        .collect::<Vec<_>>();
    let mut state = crate::state::TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: crate::inspector::LoadState::Ready(session_model),
        ..crate::state::TuiState::default()
    };
    let simple = crate::render::primary_text(&state);
    state.tool_invocation_visibility = crate::state::ToolInvocationVisibility::StructuredParameters;
    let detailed = crate::render::primary_text(&state);

    assert!(simple.contains("Find documents"));
    assert!(detailed.contains("[REDACTED]"));
    assert_eq!(
        activity_ids,
        match &state.session {
            crate::inspector::LoadState::Ready(model) => model
                .rows
                .iter()
                .map(|row| row.reference)
                .collect::<Vec<_>>(),
            _ => unreachable!(),
        }
    );
}
