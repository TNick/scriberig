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
    };
    let model = SessionModel::from_session(&session);
    assert_eq!(model.rows[0].kind, SessionRowKind::Session);
    assert_eq!(model.rows[1].depth, 1);
    assert_eq!(model.rows[2].depth, 2);
    assert_eq!(model.reference_at(2), Some(EntityId::from_bytes([3; 16])));
    assert!(model.rows[1].transient);
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
    };

    let model = SessionModel::from_session(&session);

    assert!(model.rows.iter().any(|row| {
        row.kind == SessionRowKind::Turn
            && row
                .label
                .contains("The current project is onboarded and ready.")
    }));
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
    };
    let model = SessionModel::from_session(&session);
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
