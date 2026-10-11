//! Verifies provider-neutral session structure and lifecycle.
use scriberig_core::EntityId;
use scriberig_session::{Branch, BranchStatus, Session, SessionError, Turn};

/// Checks branch ordering, cross-domain handles, and lifecycle validation.
#[test]
fn branch_structure_is_navigable() {
    let id = EntityId::new([1; 16]);
    let mut branch = Branch::new(id, "work").expect("branch");
    branch.touch(EntityId::new([8; 16]));
    branch.push_turn(Turn {
        id: EntityId::new([2; 16]),
        ordinal: 99,
        human: None,
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    assert_eq!(branch.turns[0].ordinal, 0);
    branch
        .transition(BranchStatus::Completed)
        .expect("complete");
    assert_eq!(
        branch.transition(BranchStatus::Active),
        Err(SessionError::InvalidTransition {
            from: BranchStatus::Completed,
            to: BranchStatus::Active
        })
    );
    let session = Session {
        id: EntityId::new([3; 16]),
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
    };
    assert_eq!(session.branches.len(), 1);
}
