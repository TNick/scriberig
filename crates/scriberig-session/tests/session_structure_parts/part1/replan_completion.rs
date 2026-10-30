//! Verifies replan and completed-result durability behavior.

use super::*;

/// Preserves completed result identities through explicit replan revisions.
#[test]
fn replan_preserves_completed_results_and_fulfills_new_edges() {
    let mut plan = plan_fixture();
    let completed = plan.nodes[0].id;
    let old_unfinished = plan.nodes[1].id;
    plan.add_dependency(old_unfinished, completed)
        .expect("add historical dependency");
    let dependency_revision = plan.revision;
    let result_id = EntityId::from_bytes([110; 16]);
    plan.nodes[0].status = PlanNodeStatus::Completed;
    plan.nodes[0].result = Some(result_id);
    plan.status = PlanStatus::Paused;
    let replacement = plan_node(111, plan.root_branch, "replacement");
    let replacement_id = replacement.id;
    plan.replan(
        vec![replacement],
        vec![(completed, replacement_id)],
        &[],
        &[(old_unfinished, replacement_id)],
        &[],
        &[completed],
        "replace obsolete work",
    )
    .expect("replan");
    let preserved = plan.nodes.iter().find(|node| node.id == completed).unwrap();
    assert_eq!(preserved.status, PlanNodeStatus::Completed);
    assert_eq!(preserved.result, Some(result_id));
    let edge = plan
        .dependencies
        .iter()
        .find(|edge| edge.prerequisite == completed && edge.dependent == replacement_id)
        .expect("new dependency");
    assert_eq!(edge.fulfilled_by, Some(result_id));
    assert_eq!(
        plan.nodes
            .iter()
            .find(|node| node.id == old_unfinished)
            .unwrap()
            .superseded_by,
        Some(replacement_id)
    );
    assert!(
        plan.history
            .iter()
            .find(|revision| revision.revision == dependency_revision)
            .is_some_and(|revision| revision.dependencies.iter().any(|edge| {
                edge.prerequisite == old_unfinished && edge.dependent == completed
            }))
    );
    assert!(
        !plan
            .dependencies
            .iter()
            .any(|edge| edge.prerequisite == old_unfinished && edge.dependent == completed)
    );
    let before_cycle = plan.clone();
    assert_eq!(
        plan.replan(
            vec![
                plan_node(112, plan.root_branch, "cycle attempt a"),
                plan_node(113, plan.root_branch, "cycle attempt b"),
            ],
            vec![
                (
                    EntityId::from_bytes([112; 16]),
                    EntityId::from_bytes([113; 16])
                ),
                (
                    EntityId::from_bytes([113; 16]),
                    EntityId::from_bytes([112; 16])
                ),
            ],
            &[],
            &[],
            &[],
            &[completed],
            "cycle attempt",
        ),
        Err(PlanError::DependencyCycle)
    );
    assert_eq!(plan, before_cycle);
}

/// Gates a comparison node on explicit results from isolated sibling attempts.
#[test]
fn alternative_comparison_waits_for_both_attempt_results() {
    let mut plan = plan_fixture();
    let root = plan.root_branch;
    let mut first = plan_node(120, root, "first attempt");
    first.assignment = Some(ModelAssignment {
        provider_id: "provider-a".to_owned(),
        model_id: "model-a".to_owned(),
        thinking_level: None,
    });
    let mut second = plan_node(121, root, "second attempt");
    second.assignment = Some(ModelAssignment {
        provider_id: "provider-b".to_owned(),
        model_id: "model-b".to_owned(),
        thinking_level: None,
    });
    let comparison = plan_node(122, root, "compare attempts");
    let first_id = first.id;
    let second_id = second.id;
    let comparison_id = comparison.id;
    plan.add_alternative_attempts(vec![first, second], comparison)
        .expect("alternative attempts");
    let first_result = branch_result(
        EntityId::from_bytes([123; 16]),
        EntityId::from_bytes([124; 16]),
        None,
    );
    let second_result = branch_result(
        EntityId::from_bytes([125; 16]),
        EntityId::from_bytes([126; 16]),
        None,
    );
    let first_node = plan
        .nodes
        .iter_mut()
        .find(|node| node.id == first_id)
        .unwrap();
    first_node.status = PlanNodeStatus::Completed;
    first_node.branch = Some(first_result.branch);
    first_node.result = Some(first_result.id);
    let second_node = plan
        .nodes
        .iter_mut()
        .find(|node| node.id == second_id)
        .unwrap();
    second_node.status = PlanNodeStatus::Completed;
    second_node.branch = Some(second_result.branch);
    second_node.result = Some(second_result.id);
    for edge in &mut plan.dependencies {
        edge.fulfilled_by = if edge.prerequisite == first_id {
            Some(first_result.id)
        } else if edge.prerequisite == second_id {
            Some(second_result.id)
        } else {
            edge.fulfilled_by
        };
    }
    assert!(
        !plan
            .ready_nodes_from_results(std::slice::from_ref(&first_result))
            .contains(&comparison_id)
    );
    assert!(
        plan.ready_nodes_from_results(&[first_result, second_result])
            .contains(&comparison_id)
    );
    assert_eq!(
        plan.dependencies
            .iter()
            .filter(|edge| edge.dependent == comparison_id)
            .map(|edge| edge.prerequisite)
            .collect::<Vec<_>>(),
        vec![first_id, second_id]
    );
}

/// Persists completion only after exact results exist and releases ownership.
#[test]
fn plan_completion_waits_for_results_and_releases_coordinator() {
    let root = EntityId::from_bytes([130; 16]);
    let branch = EntityId::from_bytes([131; 16]);
    let result_id = EntityId::from_bytes([132; 16]);
    let node_id = EntityId::from_bytes([133; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([134; 16]),
        EntityId::from_bytes([135; 16]),
        ContinuationAnchor::BranchHead { branch: root },
        "finish",
    )
    .expect("plan");
    plan.add_node(plan_node(133, root, "only node"))
        .expect("node");
    plan.transition(PlanStatus::Planned).expect("plan plan");
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([136; 16]), "coordinator")
        .expect("coordinator");
    let claim_id = EntityId::from_bytes([137; 16]);
    plan.start_node(node_id, branch, claim_id, "worker", &[])
        .expect("start node");
    let result = branch_result(result_id, branch, None);
    plan.complete_node(node_id, claim_id, &result)
        .expect("complete node");
    assert!(
        !plan
            .complete_if_satisfied(&[])
            .expect("missing durable result remains incomplete")
    );
    assert!(
        plan.complete_if_satisfied(&[result])
            .expect("complete with durable result")
    );
    assert_eq!(plan.status, PlanStatus::Completed);
    assert!(plan.coordinator_claim.is_none());
}

/// Commits a node result before making its dependent ready in session state.
#[test]
fn session_plan_completion_requires_persisted_branch_result() {
    let session_id = EntityId::from_bytes([90; 16]);
    let root = EntityId::from_bytes([91; 16]);
    let worker = EntityId::from_bytes([92; 16]);
    let plan_id = EntityId::from_bytes([93; 16]);
    let producer = EntityId::from_bytes([94; 16]);
    let dependent = EntityId::from_bytes([95; 16]);
    let result_id = EntityId::from_bytes([96; 16]);
    let anchor = ContinuationAnchor::BranchHead { branch: root };
    let mut plan = Plan::new(plan_id, session_id, anchor, "persist result").expect("plan");
    plan.transition(PlanStatus::Planned).expect("plan planned");
    plan.add_node(plan_node(94, root, "producer"))
        .expect("producer");
    plan.add_node(plan_node(95, root, "dependent"))
        .expect("dependent");
    plan.add_dependency(producer, dependent)
        .expect("dependency");
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([97; 16]), "coordinator")
        .expect("claim coordinator");
    let claim_id = EntityId::from_bytes([98; 16]);
    plan.start_node(producer, worker, claim_id, "worker", &[])
        .expect("start producer");
    let mut session = Session {
        id: session_id,
        branches: vec![
            Branch::new(root, "root").expect("root"),
            Branch::new(worker, "worker").expect("worker"),
        ],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };
    session.add_plan(plan).expect("add plan");
    assert_eq!(
        session.complete_plan_node(plan_id, producer, claim_id, result_id),
        Err(PlanError::ResultMismatch)
    );
    let result = branch_result(result_id, worker, None);
    session.branches[1].result = Some(result);
    session
        .complete_plan_node(plan_id, producer, claim_id, result_id)
        .expect("record result fulfillment");
    assert_eq!(session.ready_plan_nodes(plan_id), vec![dependent]);
}

/// Creates a plan fixture with two independent workers and one dependent node.
/// # Returns
/// The operation result.
pub(crate) fn plan_fixture() -> Plan {
    let root = EntityId::from_bytes([20; 16]);
    let mut plan = Plan {
        id: EntityId::from_bytes([21; 16]),
        session: None,
        anchor: None,
        source_snapshot: None,
        goal: String::new(),
        revision: 0,
        status: PlanStatus::Draft,
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
    plan.add_node(plan_node(22, root, "parser"))
        .expect("parser node");
    plan.add_node(plan_node(23, root, "docs"))
        .expect("docs node");
    plan.add_node(plan_node(24, root, "integration"))
        .expect("integration node");
    plan.transition(PlanStatus::Planned).expect("plan plan");
    plan
}
