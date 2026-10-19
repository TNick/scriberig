//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies historical continuation is inserted at its structural anchor.
#[test]
fn historical_continuation_is_inserted_at_its_structural_anchor() {
    let session_id = EntityId::from_bytes([51; 16]);
    let root_id = EntityId::from_bytes([52; 16]);
    let first_turn = EntityId::from_bytes([53; 16]);
    let later_turn = EntityId::from_bytes([54; 16]);
    let child_id = EntityId::from_bytes([55; 16]);
    let mut root = Branch::new(root_id, "Root").expect("root branch");
    for (id, ordinal) in [(first_turn, 0), (later_turn, 1)] {
        root.push_turn(scriberig_session::Turn {
            id,
            ordinal,
            human: Some(scriberig_session::HumanInput {
                text: format!("request {ordinal}"),
            }),
            agent: Some(scriberig_session::AgentOutput {
                text: format!("response {ordinal}"),
            }),
            tool_invocation: None,
            tool_result: None,
        });
    }
    let mut session = Session {
        id: session_id,
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    session
        .add_continuation_branch(
            child_id,
            scriberig_session::ContinuationAnchor::AssistantOutput {
                branch: root_id,
                turn: first_turn,
            },
            "Historical reply",
            "continue earlier response",
            scriberig_session::ContinuationOrigin::HumanContinuation,
        )
        .expect("historical continuation");

    let projection = ConversationModel::from_session(&session);
    let first = projection
        .index_of(ConversationSource::Activity(first_turn))
        .expect("anchor activity");
    let child = projection
        .index_of(ConversationSource::Branch(child_id))
        .expect("continuation branch");
    let later = projection
        .index_of(ConversationSource::Activity(later_turn))
        .expect("later activity");

    assert!(first < child);
    assert!(child < later);
}

/// Inserts a plan at its exact historical reply anchor in the same tree.
#[test]
fn historical_plan_appears_at_its_exact_anchor() {
    let session_id = EntityId::from_bytes([111; 16]);
    let branch_id = EntityId::from_bytes([112; 16]);
    let plan_id = EntityId::from_bytes([113; 16]);
    let first_turn = EntityId::from_bytes([114; 16]);
    let later_turn = EntityId::from_bytes([115; 16]);
    let mut branch = Branch::new(branch_id, "Root").expect("root");
    for (id, ordinal) in [(first_turn, 0), (later_turn, 1)] {
        branch.push_turn(scriberig_session::Turn {
            id,
            ordinal,
            human: Some(scriberig_session::HumanInput {
                text: format!("message {ordinal}"),
            }),
            agent: Some(scriberig_session::AgentOutput {
                text: format!("response {ordinal}"),
            }),
            tool_invocation: None,
            tool_result: None,
        });
    }
    let anchor = scriberig_session::ContinuationAnchor::AssistantOutput {
        branch: branch_id,
        turn: first_turn,
    };
    let mut plan = Plan::new(plan_id, session_id, anchor, "Improve first response").expect("plan");
    plan.transition(PlanStatus::Planned).expect("planned");
    let session = Session {
        id: session_id,
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let projection = ConversationModel::from_session(&session);
    let first = projection
        .index_of(ConversationSource::Activity(first_turn))
        .expect("first anchor");
    let plan = projection
        .index_of(ConversationSource::Plan(plan_id))
        .expect("plan row");
    let later = projection
        .index_of(ConversationSource::Activity(later_turn))
        .expect("later activity");

    assert!(first < plan);
    assert!(plan < later);
}

/// Derives three levels of visible ancestry from the durable branch graph.
#[test]
fn nested_branches_keep_deterministic_depth_rails() {
    let session_id = EntityId::from_bytes([121; 16]);
    let root_id = EntityId::from_bytes([122; 16]);
    let child_id = EntityId::from_bytes([123; 16]);
    let grandchild_id = EntityId::from_bytes([124; 16]);
    let mut root = Branch::new(root_id, "Root").expect("root");
    root.children.push(child_id);
    let mut child = Branch::new_child(child_id, root_id, "Worker", "inspect").expect("child");
    child.children.push(grandchild_id);
    let grandchild =
        Branch::new_child(grandchild_id, child_id, "Nested", "verify").expect("grandchild");
    let session = Session {
        id: session_id,
        branches: vec![root, child, grandchild],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let projection = ConversationModel::from_session(&session);
    let depth = |id| {
        projection
            .rows
            .iter()
            .find(|row| row.source == ConversationSource::Branch(id))
            .expect("branch row")
            .depth
    };

    assert_eq!(depth(root_id), 1);
    assert_eq!(depth(child_id), 2);
    assert_eq!(depth(grandchild_id), 3);
}

/// Expands retained completed history as presentation-only projection state.
#[test]
fn completed_child_expansion_does_not_mutate_session_state() {
    let session_id = EntityId::from_bytes([61; 16]);
    let root_id = EntityId::from_bytes([62; 16]);
    let child_id = EntityId::from_bytes([63; 16]);
    let child_turn = EntityId::from_bytes([64; 16]);
    let mut root = Branch::new(root_id, "Root").expect("root");
    root.children.push(child_id);
    let mut child = Branch::new_child(child_id, root_id, "Worker", "implement").expect("child");
    child.push_turn(scriberig_session::Turn {
        id: child_turn,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "child prompt".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    child
        .transition(scriberig_session::BranchStatus::Completed)
        .expect("complete child");
    let session = Session {
        id: session_id,
        branches: vec![root, child],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let original = session.clone();

    let compact = ConversationModel::from_session(&session);
    let expanded = ConversationModel::from_session_with_expanded(
        &session,
        &std::collections::BTreeSet::from([child_id]),
    );

    assert!(
        compact
            .index_of(ConversationSource::Activity(child_turn))
            .is_none()
    );
    assert!(
        expanded
            .index_of(ConversationSource::Activity(child_turn))
            .is_some()
    );
    assert_eq!(session, original);
}
