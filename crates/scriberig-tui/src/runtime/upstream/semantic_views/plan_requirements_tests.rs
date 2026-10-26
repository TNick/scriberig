//! Verifies the Plans view renders shared gate, evidence, blocker, and retry summaries.
//!
//! Gate counts, evidence counts, dependency blockers, summary links, and retry
//! capability all come from `scriberig-session`'s `PlanNodeView`; the terminal
//! renders them rather than deriving plan semantics of its own.

use scriberig_session::{
    PlanEvidenceOwner, PlanEvidenceRole, PlanGate, PlanGatePhase, PlanGateWaiver, PlanNodeParent,
    PlanNodeStatus, WorkerClaim,
};

use super::test_support::{
    evidence, failure_reference, node_with_status, plan_rows, required_child_gate, sample_plan,
    selected_details, session_with_results, shared_node_view, summary, validation_entry_gate,
};
use super::*;

/// Verifies gate, evidence, summary, blocker, and retry summaries come from the
/// shared node view rather than from anything the terminal computes.
#[test]
fn gate_evidence_summary_and_blocker_summaries_match_the_shared_view() {
    let plan_id = EntityId::from_bytes([21; 16]);
    let branch = EntityId::from_bytes([22; 16]);
    let prerequisite = EntityId::from_bytes([23; 16]);
    let work = EntityId::from_bytes([24; 16]);
    let entry_gate = EntityId::from_bytes([25; 16]);
    let completion_gate = EntityId::from_bytes([26; 16]);
    let done_child = EntityId::from_bytes([27; 16]);

    let mut plan = sample_plan(plan_id, branch, prerequisite, work);
    plan.nodes = vec![
        node_with_status(prerequisite, None, PlanNodeStatus::Completed),
        node_with_status(work, None, PlanNodeStatus::Running),
        node_with_status(
            done_child,
            Some(PlanNodeParent::Node(work)),
            PlanNodeStatus::Completed,
        ),
    ];
    plan.root_nodes = vec![prerequisite, work];
    plan.dependencies = vec![scriberig_session::PlanDependency {
        prerequisite,
        dependent: work,
        fulfilled_by: None,
    }];
    plan.gates = vec![
        validation_entry_gate(entry_gate, work),
        required_child_gate(completion_gate, work, done_child),
    ];
    plan.evidence = vec![
        evidence(
            EntityId::from_bytes([28; 16]),
            PlanEvidenceOwner::Node(work),
            PlanEvidenceRole::Informational,
            EntityId::from_bytes([31; 16]),
        ),
        evidence(
            EntityId::from_bytes([29; 16]),
            PlanEvidenceOwner::Gate(entry_gate),
            PlanEvidenceRole::Informational,
            EntityId::from_bytes([32; 16]),
        ),
    ];
    let evidence_snapshot = scriberig_core::SnapshotId::from_bytes([178; 32]);
    plan.evidence[0].evidence.reference.snapshot = Some(evidence_snapshot);
    plan.summaries = vec![summary(
        EntityId::from_bytes([30; 16]),
        work,
        "Parser is half merged; blockers tracked.",
    )];
    let (rows, session) = plan_rows(plan.clone());

    let view = shared_node_view(&session, &plan, work);
    assert_eq!(view.entry_gates.pending, 1);
    assert_eq!(view.completion_gates.satisfied, 1);
    assert_eq!(view.evidence_count, 2);
    assert_eq!(view.dependency_blockers, vec![prerequisite]);

    let work_row = rows
        .iter()
        .find(|row| row.selection.node == Some(work))
        .expect("work node row");
    assert!(
        work_row.label.contains("2 gates · 2 evidence"),
        "the compact row carries the shared gate and evidence counts: {:?}",
        work_row.label
    );

    let details = selected_details(&session, &rows, plan_id, Some(work));
    for expected in [
        "Entry gates: pending 1 · satisfied 0 · failed 0 · waived 0",
        "Completion gates: pending 0 · satisfied 1 · failed 0 · waived 0",
        "Evidence: 2",
        &format!("Blocked by: {prerequisite}"),
        "Summary: Parser is half merged; blockers tracked.",
        &format!(
            "inspect \\`/inspect {} --snapshot {evidence_snapshot}\\`",
            plan.evidence[0].evidence.reference.entity
        ),
        &format!(
            "Summary identity: {}",
            view.summary_id.expect("summary identity")
        ),
    ] {
        assert!(
            details.contains(expected),
            "missing {expected:?} in {details}"
        );
    }
    assert!(details.contains("Retry: not available"), "{details}");
}

/// Verifies retry capability and retained failure follow the shared node
/// view's values, including the change a retained worker claim causes.
#[test]
fn retry_capability_and_retained_failure_follow_the_shared_view() {
    let plan_id = EntityId::from_bytes([41; 16]);
    let branch = EntityId::from_bytes([42; 16]);
    let work = EntityId::from_bytes([43; 16]);
    let failed_result = EntityId::from_bytes([44; 16]);
    let work_branch = EntityId::from_bytes([45; 16]);

    let mut plan = sample_plan(plan_id, branch, work, work);
    plan.nodes = vec![node_with_status(work, None, PlanNodeStatus::Failed)];
    plan.root_nodes = vec![work];
    plan.dependencies = Vec::new();
    plan.nodes[0].branch = Some(work_branch);
    plan.nodes[0].last_failure = Some(failure_reference(failed_result));
    let (rows, session) = plan_rows(plan.clone());

    let view = shared_node_view(&session, &plan, work);
    assert!(view.can_retry, "a claimed-free failed node is retryable");

    let details = selected_details(&session, &rows, plan_id, Some(work));
    assert!(details.contains("Retry: available"), "{details}");
    assert!(
        details.contains(&format!("Last failure: {failed_result} · no snapshot")),
        "{details}"
    );

    let mut claimed = plan.clone();
    claimed.nodes[0].claim = Some(WorkerClaim {
        id: EntityId::from_bytes([46; 16]),
        node: work,
        branch: work_branch,
        owner: "worker-a".to_owned(),
        generation: 3,
        assignment: None,
    });
    let (claimed_rows, claimed_session) = plan_rows(claimed.clone());
    assert!(
        !shared_node_view(&claimed_session, &claimed, work).can_retry,
        "a retained worker claim blocks retry"
    );
    let claimed_details = selected_details(&claimed_session, &claimed_rows, plan_id, Some(work));
    assert!(
        claimed_details.contains("Retry: not available"),
        "{claimed_details}"
    );
    assert!(
        claimed_details.contains("Worker claim:"),
        "the canonical claim stays inspectable: {claimed_details}"
    );
}

/// Verifies shared dependency blockers stay detail-pane data while the unfulfilled
/// dependent row keeps the plan header as its parent.
#[test]
fn shared_dependency_blockers_stay_outside_tree_parentage() {
    let plan_id = EntityId::from_bytes([61; 16]);
    let branch = EntityId::from_bytes([62; 16]);
    let prerequisite = EntityId::from_bytes([63; 16]);
    let dependent = EntityId::from_bytes([64; 16]);
    let result = EntityId::from_bytes([65; 16]);

    let mut plan = sample_plan(plan_id, branch, prerequisite, dependent);
    plan.nodes = vec![
        node_with_status(prerequisite, None, PlanNodeStatus::Completed),
        node_with_status(dependent, None, PlanNodeStatus::Planned),
    ];
    plan.root_nodes = vec![prerequisite];
    plan.dependencies = vec![scriberig_session::PlanDependency {
        prerequisite,
        dependent,
        fulfilled_by: Some(result),
    }];
    let (rows, session) = plan_rows(plan.clone());

    let view = shared_node_view(&session, &plan, dependent);
    assert_eq!(
        view.dependency_blockers,
        vec![prerequisite],
        "the unfulfilled edge is reported as a blocker"
    );

    let header = rows
        .iter()
        .find(|row| row.selection.node.is_none())
        .expect("plan header row");
    let dependent_row = rows
        .iter()
        .find(|row| row.selection.node == Some(dependent))
        .expect("dependent node row");
    let prerequisite_row = rows
        .iter()
        .find(|row| row.selection.node == Some(prerequisite))
        .expect("prerequisite node row");
    assert_eq!(dependent_row.parent_id.as_deref(), Some(header.id.as_str()));
    assert_ne!(
        dependent_row.parent_id.as_deref(),
        Some(prerequisite_row.id.as_str()),
        "a shared-view blocker must not become structural parentage"
    );

    let details = selected_details(&session, &rows, plan_id, Some(dependent));
    assert!(
        details.contains(&format!("Blocked by: {prerequisite}")),
        "{details}"
    );
    assert!(
        details.contains(&format!("{prerequisite} → {dependent} · result {result}")),
        "the edge itself stays plan knowledge in the details: {details}"
    );
}

/// Verifies a waived gate is still counted and reported as visibly waived, so
/// the terminal never hides gate provenance.
#[test]
fn a_waived_gate_is_rendered_as_waived_from_the_shared_view() {
    let plan_id = EntityId::from_bytes([91; 16]);
    let branch = EntityId::from_bytes([92; 16]);
    let work = EntityId::from_bytes([93; 16]);
    let gate = EntityId::from_bytes([94; 16]);

    let mut plan = sample_plan(plan_id, branch, work, work);
    plan.nodes = vec![node_with_status(work, None, PlanNodeStatus::Ready)];
    plan.root_nodes = vec![work];
    plan.dependencies = Vec::new();
    plan.gates = vec![PlanGate {
        id: gate,
        owner: work,
        phase: PlanGatePhase::Entry,
        requirement: scriberig_session::PlanGateRequirement::HumanApproval {
            policy: "release".to_owned(),
            role: None,
        },
        waiver: Some(PlanGateWaiver {
            reason: "Accepted by the release owner.".to_owned(),
            actor: None,
            operation: None,
        }),
    }];
    let (rows, session) = plan_rows(plan.clone());

    let view = shared_node_view(&session, &plan, work);
    assert_eq!(view.entry_gates.waived, 1);

    let details = selected_details(&session, &rows, plan_id, Some(work));
    assert!(
        details.contains("Entry gates: pending 0 · satisfied 0 · failed 0 · waived 1"),
        "{details}"
    );
    let work_row = rows
        .iter()
        .find(|row| row.selection.node == Some(work))
        .expect("work node row");
    assert!(
        work_row.label.contains("1 gate"),
        "the visible row keeps the waived gate count: {:?}",
        work_row.label
    );
}

/// Exposes pending and failed gate definitions and evaluator reasons in node details.
#[test]
fn pending_and_failed_gate_details_include_typed_requirements() {
    let plan_id = EntityId::from_bytes([111; 16]);
    let branch = EntityId::from_bytes([112; 16]);
    let root = EntityId::from_bytes([113; 16]);
    let child = EntityId::from_bytes([114; 16]);
    let pending_gate = EntityId::from_bytes([115; 16]);
    let failed_gate = EntityId::from_bytes([116; 16]);

    let mut plan = sample_plan(plan_id, branch, root, child);
    plan.nodes = vec![
        node_with_status(root, None, PlanNodeStatus::Ready),
        node_with_status(
            child,
            Some(PlanNodeParent::Node(root)),
            PlanNodeStatus::Failed,
        ),
    ];
    plan.root_nodes = vec![root];
    plan.dependencies.clear();
    plan.gates = vec![
        validation_entry_gate(pending_gate, root),
        required_child_gate(failed_gate, root, child),
    ];
    let (rows, session) = plan_rows(plan);

    let details = selected_details(&session, &rows, plan_id, Some(root));

    assert!(
        details.contains(&format!("Entry gate {pending_gate} · Pending")),
        "{details}"
    );
    assert!(details.contains("validation passed"), "{details}");
    assert!(
        details.contains(&format!("Completion gate {failed_gate} · Failed")),
        "{details}"
    );
    assert!(
        details.contains("required child is not complete"),
        "{details}"
    );
    assert!(
        details.contains(&format!("required child completed: {child}")),
        "{details}"
    );
}

/// Verifies fulfilled dependencies use the snapshot's branch results during
/// retry evaluation, matching the shared application plan read.
#[test]
fn fulfilled_dependencies_use_the_snapshot_branch_results() {
    let plan_id = EntityId::from_bytes([101; 16]);
    let branch = EntityId::from_bytes([102; 16]);
    let prerequisite = EntityId::from_bytes([103; 16]);
    let dependent = EntityId::from_bytes([104; 16]);
    let result = EntityId::from_bytes([105; 16]);

    let mut plan = sample_plan(plan_id, branch, prerequisite, dependent);
    plan.nodes = vec![
        node_with_status(prerequisite, None, PlanNodeStatus::Completed),
        node_with_status(dependent, None, PlanNodeStatus::Failed),
    ];
    plan.root_nodes = vec![prerequisite];
    plan.dependencies = vec![scriberig_session::PlanDependency {
        prerequisite,
        dependent,
        fulfilled_by: Some(result),
    }];
    plan.nodes[0].branch = Some(result);
    plan.nodes[0].result = Some(result);
    plan.nodes[1].branch = Some(branch);

    let session = session_with_results(vec![plan.clone()], std::slice::from_ref(&result));
    let rows = rows_for_session(&session);
    let view = shared_node_view(&session, &plan, dependent);
    assert!(
        view.dependency_blockers.is_empty(),
        "{:?}",
        view.dependency_blockers
    );
    assert!(
        view.can_retry,
        "the failed node is retryable once its input is fulfilled"
    );

    let details = selected_details(&session, &rows, plan_id, Some(dependent));
    assert!(details.contains("Blocked by: none"), "{details}");
    assert!(details.contains("Retry: available"), "{details}");
}
