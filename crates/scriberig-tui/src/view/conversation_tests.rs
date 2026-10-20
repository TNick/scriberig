//! Verifies stable conversation projections over durable session and plan state.

use super::{ConversationModel, ConversationRowKind, ConversationSource};
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchResult, BranchStatus, ContinuationAnchor, ContinuationOrigin, Plan, PlanNode,
    PlanNodeParent, PlanNodeStatus, PlanStatus, Session, Turn,
};

/// Shows accepted user text and its durable failure result in conversation order.
#[test]
fn failed_first_turn_projects_message_followed_by_readable_error() {
    let session_id = EntityId::from_bytes([31; 16]);
    let branch_id = EntityId::from_bytes([32; 16]);
    let turn_id = EntityId::from_bytes([33; 16]);
    let result_id = EntityId::from_bytes([34; 16]);
    let mut branch = Branch::new(branch_id, "agent root").expect("root branch");
    branch.push_turn(Turn {
        id: turn_id,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "1+1".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    branch
        .activity
        .push(scriberig_session::ExecutionActivity::HumanInput(turn_id));
    branch.result = Some(BranchResult {
        id: result_id,
        branch: branch_id,
        status: BranchStatus::Abandoned,
        summary: Some("provider connection failed".to_owned()),
        result_kind: Some("agent-failure".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    branch
        .transition(BranchStatus::Abandoned)
        .expect("failed branch transition");
    let session = Session {
        id: session_id,
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let projection = ConversationModel::from_session(&session);
    let message = projection
        .rows
        .iter()
        .position(|row| row.source == ConversationSource::Activity(turn_id))
        .expect("human message row");
    let error = projection
        .rows
        .iter()
        .position(|row| row.source == ConversationSource::BranchResult(result_id))
        .expect("failure result row");

    assert!(projection.rows[message].label.contains("You: 1+1"));
    assert!(
        projection.rows[error]
            .label
            .contains("Error: provider connection failed")
    );
    assert!(message < error);
}

/// Projects plan state as stable rows while keeping the session canonical.
#[test]
fn projects_plan_nodes_with_stable_sources_and_dependency_state() {
    let session_id = EntityId::from_bytes([41; 16]);
    let branch_id = EntityId::from_bytes([42; 16]);
    let plan_id = EntityId::from_bytes([43; 16]);
    let first_id = EntityId::from_bytes([45; 16]);
    let second_id = EntityId::from_bytes([44; 16]);
    let third_id = EntityId::from_bytes([46; 16]);
    let mut branch = Branch::new(branch_id, "Root").expect("root branch");
    branch.children = Vec::new();
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "Build parser",
    )
    .expect("plan");
    for (id, title) in [
        (first_id, "Inspect"),
        (second_id, "Design"),
        (third_id, "Implement"),
    ] {
        plan.add_node(PlanNode {
            id,
            parent: Some(PlanNodeParent::Branch(branch_id)),
            title: title.to_owned(),
            purpose: title.to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: PlanNodeStatus::Planned,
            branch: None,
            result: None,
            superseded_by: None,
            assignment: None,
            claim: None,
            claim_generation: 0,
        })
        .expect("node");
    }
    plan.add_dependency(first_id, third_id).expect("dependency");
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

    assert!(
        projection
            .index_of(ConversationSource::Plan(plan_id))
            .is_some()
    );
    let dependency = projection
        .rows
        .iter()
        .find(|row| row.source == ConversationSource::PlanNode(third_id))
        .expect("dependent node");
    assert_eq!(dependency.kind, ConversationRowKind::PlanNode);
    assert!(dependency.label.contains("waits: "));
    assert!(
        projection.index_of(ConversationSource::PlanNode(first_id))
            < projection.index_of(ConversationSource::PlanNode(third_id))
    );
    assert_eq!(session.plans[0].dependencies.len(), 1);
    for status in [
        PlanNodeStatus::Planned,
        PlanNodeStatus::Ready,
        PlanNodeStatus::Running,
        PlanNodeStatus::Completed,
    ] {
        let mut state = session.clone();
        state.plans[0].nodes[0].status = status;
        let projected = ConversationModel::from_session(&state);
        let row = projected
            .rows
            .iter()
            .find(|row| row.source == ConversationSource::PlanNode(first_id))
            .expect("same stable node row");
        assert!(row.label.contains(&format!("{status:?}")));
    }
}

/// Nests an executing plan worker branch beneath its owning plan node.
#[test]
fn plan_worker_branch_renders_under_its_stable_plan_node() {
    let session_id = EntityId::from_bytes([51; 16]);
    let root_id = EntityId::from_bytes([52; 16]);
    let worker_id = EntityId::from_bytes([53; 16]);
    let plan_id = EntityId::from_bytes([54; 16]);
    let node_id = EntityId::from_bytes([55; 16]);
    let second_worker_id = EntityId::from_bytes([56; 16]);
    let second_node_id = EntityId::from_bytes([57; 16]);
    let mut root = Branch::new(root_id, "Root").expect("root branch");
    root.children.extend([worker_id, second_worker_id]);
    let worker = Branch::new_child(worker_id, root_id, "Implement", "implement parser")
        .expect("worker branch");
    let second_worker = Branch::new_child(second_worker_id, root_id, "Document", "document parser")
        .expect("second worker branch");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: root_id },
        "Build parser",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_id)),
        title: "Implement".to_owned(),
        purpose: "implement parser".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Running,
        branch: Some(worker_id),
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 1,
    })
    .expect("node");
    plan.add_node(PlanNode {
        id: second_node_id,
        parent: Some(PlanNodeParent::Branch(root_id)),
        title: "Document".to_owned(),
        purpose: "document parser".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Running,
        branch: Some(second_worker_id),
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 1,
    })
    .expect("second node");
    plan.transition(PlanStatus::Planned).expect("planned");
    let session = Session {
        id: session_id,
        branches: vec![root, worker, second_worker],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let projection = ConversationModel::from_session(&session);
    let node_index = projection
        .index_of(ConversationSource::PlanNode(node_id))
        .expect("plan node row");
    let worker_index = projection
        .index_of(ConversationSource::Branch(worker_id))
        .expect("worker branch row");
    assert!(node_index < worker_index);
    assert_eq!(
        projection.rows[worker_index].depth,
        projection.rows[node_index].depth + 1
    );
    let second_node_index = projection
        .index_of(ConversationSource::PlanNode(second_node_id))
        .expect("second plan node row");
    let second_worker_index = projection
        .index_of(ConversationSource::Branch(second_worker_id))
        .expect("second worker branch row");
    assert!(second_node_index < second_worker_index);
    assert_eq!(
        projection.rows[second_worker_index].depth,
        projection.rows[second_node_index].depth + 1
    );
}

/// Attaches live worker status below its semantic branch identity.
#[test]
fn transient_activity_is_inserted_at_its_structural_branch_location() {
    let session_id = EntityId::from_bytes([46; 16]);
    let root_id = EntityId::from_bytes([47; 16]);
    let child_id = EntityId::from_bytes([48; 16]);
    let mut root = Branch::new(root_id, "Root").expect("root");
    root.children.push(child_id);
    let child = Branch::new_child(child_id, root_id, "Worker", "implement parser").expect("child");
    let session = Session {
        id: session_id,
        branches: vec![root, child],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut model = ConversationModel::from_session(&session);

    model.insert_transient_activity(child_id, 71, "Live RuntimeStatus: working".to_owned());

    let worker = model
        .index_of(ConversationSource::Branch(child_id))
        .expect("worker branch");
    let live = worker + 1;
    let source = model.rows[live].source;
    assert_eq!(live, worker + 1);
    assert_eq!(model.rows[live].kind, ConversationRowKind::Activity);
    assert_eq!(model.rows[live].depth, model.rows[worker].depth + 1);

    let mut rerendered = ConversationModel::from_session(&session);
    rerendered.insert_transient_activity(child_id, 71, "Live RuntimeStatus: working".to_owned());
    assert_eq!(rerendered.rows[live].source, source);
}

/// Projects an autonomous agent-created child at its structural parent location.
#[test]
fn autonomous_child_branch_is_visible_in_default_conversation_tree() {
    let session_id = EntityId::from_bytes([61; 16]);
    let root_id = EntityId::from_bytes([62; 16]);
    let child_id = EntityId::from_bytes([63; 16]);
    let result_id = EntityId::from_bytes([64; 16]);
    let turn_id = EntityId::from_bytes([65; 16]);
    let mut root = Branch::new(root_id, "Root").expect("root branch");
    root.children.push(child_id);
    root.push_turn(scriberig_session::Turn {
        id: turn_id,
        ordinal: 1,
        human: Some(scriberig_session::HumanInput {
            text: "decompose this work".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "delegating implementation".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    let mut child =
        Branch::new_child(child_id, root_id, "Implement", "implement module").expect("agent child");
    child.continuation = Some(scriberig_session::ContinuationProvenance {
        anchor: ContinuationAnchor::AssistantOutput {
            branch: root_id,
            turn: turn_id,
        },
        origin: ContinuationOrigin::AgentDelegation,
    });
    child.result = Some(BranchResult {
        id: result_id,
        branch: child_id,
        status: BranchStatus::Completed,
        summary: Some("module implemented".to_owned()),
        result_kind: Some("implementation-summary".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec!["entity:module".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    let session = Session {
        id: session_id,
        branches: vec![root, child],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    let projection = ConversationModel::from_session(&session);

    let root_row = projection
        .index_of(ConversationSource::Branch(root_id))
        .expect("root row");
    let child_row = projection
        .index_of(ConversationSource::Branch(child_id))
        .expect("agent-created child row");
    let result_row = projection
        .index_of(ConversationSource::BranchResult(result_id))
        .expect("compact child result row");
    assert!(root_row < child_row);
    assert!(child_row < result_row);
    assert!(projection.rows[child_row].depth > projection.rows[root_row].depth);
    assert!(
        projection.rows[result_row]
            .label
            .contains("module implemented")
    );
}

#[path = "conversation_tests_part1.rs"]
mod part1;
