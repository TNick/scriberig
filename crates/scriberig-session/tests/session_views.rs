//! Verifies deterministic progressive session views.
use scriberig_core::EntityId;
use scriberig_session::{Branch, Session, SessionExecutionSelection, SessionViews, Turn};

/// Checks compact branch metadata and chronological turn views.
#[test]
fn views_do_not_require_turn_bodies() {
    let mut branch = Branch::new(EntityId::from_bytes([2; 16]), "branch").expect("branch");
    branch.push_turn(Turn {
        id: EntityId::from_bytes([4; 16]),
        ordinal: 7,
        human: None,
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    let session = Session {
        id: EntityId::from_bytes([1; 16]),
        branches: vec![branch.clone()],
        goals: Vec::new(),
        tasks: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };
    assert_eq!(SessionViews::branches(&session)[0].turn_count, 1);
    assert_eq!(SessionViews::turns(&branch)[0].ordinal, 0);
}
