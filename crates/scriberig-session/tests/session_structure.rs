//! Verifies provider-neutral session structure and lifecycle.
use scriberig_core::EntityId;
use scriberig_session::{
    AgentOutput, Branch, BranchResult, BranchStatus, ContinuationAnchor, ContinuationOrigin,
    ModelAssignment, Plan, PlanError, PlanNode, PlanNodeParent, PlanNodeStatus, PlanStatus,
    Session, SessionError, SessionExecutionSelection, Turn,
};

/// Builds deterministic plan nodes for scheduling tests.
/// # Parameters
/// - `id`: Input for this operation.
/// - `parent`: Input for this operation.
/// - `title`: Input for this operation.
/// # Returns
/// The operation result.
fn plan_node(id: u8, parent: EntityId, title: &str) -> PlanNode {
    PlanNode {
        id: EntityId::from_bytes([id; 16]),
        parent: Some(PlanNodeParent::Branch(parent)),
        title: title.to_owned(),
        purpose: title.to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}

/// Builds a durable successful result for plan dependency tests.
/// # Parameters
/// - `id`: Input for this operation.
/// - `branch`: Input for this operation.
/// - `kind`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn branch_result(id: EntityId, branch: EntityId, kind: Option<&str>) -> BranchResult {
    BranchResult {
        id,
        branch,
        status: BranchStatus::Completed,
        summary: Some("done".to_owned()),
        result_kind: kind.map(str::to_owned),
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec!["semantic:result".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    }
}

/// Checks exact anchor resolution and historical continuation isolation.
#[test]
fn continuation_anchors_resolve_exact_replyable_history() {
    let root_id = EntityId::from_bytes([30; 16]);
    let first_id = EntityId::from_bytes([31; 16]);
    let later_id = EntityId::from_bytes([32; 16]);
    let child_id = EntityId::from_bytes([33; 16]);
    let mut root = Branch::new(root_id, "root").expect("root");
    root.push_turn(Turn {
        id: first_id,
        ordinal: 0,
        human: None,
        agent: Some(AgentOutput {
            text: "first".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    root.push_turn(Turn {
        id: later_id,
        ordinal: 0,
        human: None,
        agent: Some(AgentOutput {
            text: "later sibling history".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    let mut session = Session {
        id: EntityId::from_bytes([34; 16]),
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };

    let anchor = ContinuationAnchor::AssistantOutput {
        branch: root_id,
        turn: first_id,
    };
    assert_eq!(
        session.resolve_anchor(anchor).expect("anchor").turn_count,
        1
    );
    session
        .add_continuation_branch(
            child_id,
            anchor,
            "historical reply",
            "continue from first output",
            ContinuationOrigin::HumanContinuation,
        )
        .expect("continuation branch");
    let resolved = session
        .resolve_anchor(anchor)
        .expect("same historical point");
    assert_eq!(resolved.turn_count, 1);
    assert_eq!(session.branches[0].turns.len(), 2);
    assert_eq!(session.branches[1].continuation.unwrap().anchor, anchor);
    assert_eq!(
        session.branches[1].continuation.unwrap().origin,
        ContinuationOrigin::HumanContinuation
    );
    assert_eq!(
        session.resolve_anchor(ContinuationAnchor::AssistantOutput {
            branch: root_id,
            turn: EntityId::from_bytes([35; 16]),
        }),
        Err(SessionError::MissingAnchorTarget)
    );
    assert_eq!(
        session.resolve_anchor(ContinuationAnchor::BranchHead {
            branch: EntityId::from_bytes([36; 16]),
        }),
        Err(SessionError::MissingAnchorBranch)
    );
}

/// Rejects a turn without user or assistant content as a continuation target.
#[test]
fn continuation_anchor_rejects_non_replyable_turn() {
    let branch_id = EntityId::from_bytes([40; 16]);
    let turn_id = EntityId::from_bytes([41; 16]);
    let mut branch = Branch::new(branch_id, "root").expect("root");
    branch.push_turn(Turn {
        id: turn_id,
        ordinal: 0,
        human: None,
        agent: None,
        tool_invocation: Some(EntityId::from_bytes([42; 16])),
        tool_result: None,
    });
    let session = Session {
        id: EntityId::from_bytes([43; 16]),
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };
    assert_eq!(
        session.resolve_anchor(ContinuationAnchor::AssistantOutput {
            branch: branch_id,
            turn: turn_id,
        }),
        Err(SessionError::NonReplyableAnchor)
    );
}

/// Ensures continuation identity uses serializable semantic handles.
#[test]
fn continuation_anchor_serializes_without_projection_coordinates() {
    let anchor = ContinuationAnchor::BranchHead {
        branch: EntityId::from_bytes([50; 16]),
    };
    let encoded = serde_json::to_string(&anchor).expect("serialize anchor");
    let decoded: ContinuationAnchor = serde_json::from_str(&encoded).expect("deserialize anchor");
    assert_eq!(decoded, anchor);
    assert!(!encoded.contains("row"));
}

/// Checks durable plan ownership, anchor, revision, and terminal lifecycle.
#[test]
fn plan_has_exact_owner_anchor_and_terminal_state() {
    let session_id = EntityId::from_bytes([60; 16]);
    let branch_id = EntityId::from_bytes([61; 16]);
    let plan_id = EntityId::from_bytes([62; 16]);
    let mut session = Session {
        id: session_id,
        branches: vec![Branch::new(branch_id, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };
    let anchor = ContinuationAnchor::BranchHead { branch: branch_id };
    let mut plan = Plan::new(plan_id, session_id, anchor, "ship feature").expect("plan");
    assert_eq!(plan.revision, 0);
    assert_eq!(plan.goal, "ship feature");
    assert_eq!(plan.anchor, Some(anchor));
    plan.transition(PlanStatus::Planned).expect("plan is ready");
    assert_eq!(plan.revision, 1);
    session.add_plan(plan.clone()).expect("owned plan");
    plan.transition(PlanStatus::Running).expect("start plan");
    plan.transition(PlanStatus::Completed).expect("finish plan");
    assert_eq!(
        plan.transition(PlanStatus::Running),
        Err(PlanError::TerminalPlan)
    );
    assert_eq!(session.plans[0].anchor, Some(anchor));
}

/// Verifies transitive dependencies and deterministic serialization ordering.
#[test]
fn plan_dependency_chain_is_valid_and_sorted() {
    let root = EntityId::from_bytes([70; 16]);
    let mut plan = Plan {
        id: EntityId::from_bytes([71; 16]),
        source_snapshot: None,
        session: None,
        anchor: None,
        goal: String::new(),
        revision: 0,
        status: PlanStatus::Planned,
        root_branch: root,
        nodes: Vec::new(),
        dependencies: Vec::new(),
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        coordinator: None,
        coordinator_cursor: None,
        coordinator_claim: None,
        coordinator_generation: 0,
        root_nodes: Vec::new(),
        history: Vec::new(),
    };
    let first = EntityId::from_bytes([72; 16]);
    let second = EntityId::from_bytes([73; 16]);
    let third = EntityId::from_bytes([74; 16]);
    plan.add_node(plan_node(72, root, "first")).expect("first");
    plan.add_node(plan_node(73, root, "second"))
        .expect("second");
    plan.add_node(plan_node(74, root, "third")).expect("third");
    plan.add_dependency(first, second).expect("first edge");
    plan.add_dependency(second, third).expect("second edge");
    assert_eq!(
        plan.add_dependency(third, first),
        Err(PlanError::DependencyCycle)
    );
    assert_eq!(plan.dependencies[0].prerequisite, first);
    assert_eq!(plan.dependencies[1].prerequisite, second);
}

/// Enforces one coordinator and isolated concurrent sibling worker claims.
#[test]
fn plan_claims_allow_one_coordinator_and_distinct_workers() {
    let mut plan = plan_fixture();
    let parser = EntityId::from_bytes([22; 16]);
    let docs = EntityId::from_bytes([23; 16]);
    let parser_branch = EntityId::from_bytes([28; 16]);
    let docs_branch = EntityId::from_bytes([29; 16]);
    let coordinator = EntityId::from_bytes([30; 16]);
    let assignment = ModelAssignment {
        provider_id: "provider-a".to_owned(),
        model_id: "model-a".to_owned(),
        thinking_level: Some("deep".to_owned()),
    };
    plan.nodes[0].assignment = Some(assignment.clone());
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(coordinator, "runtime-a")
        .expect("claim coordinator");
    assert_eq!(
        plan.claim_coordinator(EntityId::from_bytes([31; 16]), "runtime-b"),
        Err(PlanError::InvalidTransition)
    );
    let parser_claim = EntityId::from_bytes([32; 16]);
    plan.start_node(parser, parser_branch, parser_claim, "worker-a", &[])
        .expect("claim parser");
    assert_eq!(
        plan.nodes[0].claim.as_ref().unwrap().assignment,
        Some(assignment.clone())
    );
    assert_eq!(
        plan.set_assignment(parser, None),
        Err(PlanError::InvalidTransition)
    );
    assert_eq!(
        plan.start_node(
            parser,
            EntityId::from_bytes([33; 16]),
            EntityId::from_bytes([34; 16]),
            "worker-b",
            &[]
        ),
        Err(PlanError::InvalidTransition)
    );
    assert_eq!(
        plan.start_node(
            docs,
            parser_branch,
            EntityId::from_bytes([36; 16]),
            "worker-d",
            &[]
        ),
        Err(PlanError::InvalidTransition)
    );
    plan.start_node(
        docs,
        docs_branch,
        EntityId::from_bytes([35; 16]),
        "worker-c",
        &[],
    )
    .expect("claim sibling concurrently");
    plan.fail_node(parser, parser_claim)
        .expect("release failed worker");
    assert_eq!(
        plan.nodes
            .iter()
            .find(|node| node.id == parser)
            .unwrap()
            .status,
        PlanNodeStatus::Failed
    );
    plan.transition(PlanStatus::Paused).expect("pause");
    assert!(plan.coordinator_claim.is_none());
    plan.transition(PlanStatus::Running).expect("resume");
    plan.claim_coordinator(coordinator, "runtime-a")
        .expect("reclaim after resume");
    plan.transition(PlanStatus::Cancelled).expect("cancel");
    assert!(plan.coordinator_claim.is_none());
}

#[path = "session_structure_parts/part1.rs"]
mod part1;
use part1::plan_fixture;
