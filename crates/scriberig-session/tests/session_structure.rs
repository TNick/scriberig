//! Verifies provider-neutral session structure and lifecycle.
use scriberig_core::EntityId;
use scriberig_session::{Branch, BranchStatus, Session, SessionError, Turn};

/// Checks branch ordering, cross-domain handles, and lifecycle validation.
#[test]
fn branch_structure_is_navigable() {
    let id = EntityId::from_bytes([1; 16]);
    let mut branch = Branch::new(id, "work").expect("branch");
    branch.touch(EntityId::from_bytes([8; 16]));
    branch.push_turn(Turn {
        id: EntityId::from_bytes([2; 16]),
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
        id: EntityId::from_bytes([3; 16]),
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
    };
    assert_eq!(session.branches.len(), 1);
}

/// Checks that child insertion maintains both directions of the graph edge.
#[test]
fn child_branch_insertion_is_validated_and_linked() {
    let root_id = EntityId::from_bytes([10; 16]);
    let child_id = EntityId::from_bytes([11; 16]);
    let mut session = Session {
        id: EntityId::from_bytes([12; 16]),
        branches: vec![Branch::new(root_id, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
    };

    session
        .add_child_branch(child_id, root_id, "child", "inspect")
        .expect("child");
    assert_eq!(session.branches[0].id, root_id);
    assert_eq!(session.branches[0].children, vec![child_id]);
    assert_eq!(session.branches[1].parent, Some(root_id));
    assert_eq!(session.branches[1].purpose.as_deref(), Some("inspect"));
    assert_eq!(
        session.add_child_branch(child_id, root_id, "duplicate", "again"),
        Err(SessionError::DuplicateBranch)
    );
    assert_eq!(
        session.add_child_branch(
            EntityId::from_bytes([13; 16]),
            EntityId::from_bytes([14; 16]),
            "orphan",
            "none"
        ),
        Err(SessionError::MissingParent)
    );
}
