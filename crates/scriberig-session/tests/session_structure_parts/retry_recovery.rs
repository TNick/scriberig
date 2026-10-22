//! Verifies retry gates, stale claims, and terminal recovery boundaries.

use super::*;

/// Rejects a failed node whose previous worker claim has not settled.
#[test]
fn retry_rejects_a_failed_node_with_a_retained_claim() {
    let mut plan = plan_fixture();
    let node = plan.nodes[0].id;
    plan.transition(PlanStatus::Running).expect("run plan");
    let branch = EntityId::from_bytes([237; 16]);
    let failed = plan.nodes.iter_mut().find(|item| item.id == node).unwrap();
    failed.status = PlanNodeStatus::Failed;
    failed.branch = Some(branch);
    failed.claim = Some(scriberig_session::WorkerClaim {
        id: EntityId::from_bytes([238; 16]),
        node,
        branch,
        owner: "stale-worker".to_owned(),
        generation: 1,
        assignment: None,
    });

    assert_eq!(
        plan.retry_failed_node(node, &[]),
        Err(PlanError::InvalidTransition)
    );
}

/// Refuses retry when an entry gate remains unsatisfied.
#[test]
fn failed_plan_node_retry_requires_entry_gates() {
    let mut plan = plan_fixture();
    let node = plan.nodes[0].id;
    plan.add_gate(PlanGate {
        id: EntityId::from_bytes([235; 16]),
        owner: node,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::ValidationResult,
        },
        waiver: None,
    })
    .expect("add entry gate");
    plan.transition(PlanStatus::Running).expect("run plan");
    let failed = plan.nodes.iter_mut().find(|item| item.id == node).unwrap();
    failed.status = PlanNodeStatus::Failed;
    failed.branch = Some(EntityId::from_bytes([236; 16]));

    assert_eq!(
        plan.retry_failed_node(node, &[]),
        Err(PlanError::InvalidTransition)
    );
    assert_eq!(
        plan.nodes
            .iter()
            .find(|item| item.id == node)
            .unwrap()
            .status,
        PlanNodeStatus::Failed
    );
}

/// Requires stale worker claims to be recovered before a failed-node retry.
#[test]
fn retry_after_stale_claim_recovery_uses_a_fresh_branch_binding() {
    let mut plan = plan_fixture();
    let node = plan.nodes[0].id;
    let branch = EntityId::from_bytes([232; 16]);
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([233; 16]), "coordinator")
        .expect("claim coordinator");
    plan.start_node(node, branch, EntityId::from_bytes([234; 16]), "worker", &[])
        .expect("start worker");

    assert_eq!(
        plan.recover_stale_claims(|_| false)
            .expect("recover stale claims"),
        2
    );
    let failed = plan.nodes.iter().find(|item| item.id == node).unwrap();
    assert_eq!(failed.status, PlanNodeStatus::Failed);
    assert!(failed.claim.is_none());
    assert_eq!(failed.branch, Some(branch));

    plan.retry_failed_node(node, &[])
        .expect("retry after stale claim recovery");

    let retried = plan.nodes.iter().find(|item| item.id == node).unwrap();
    assert_eq!(retried.status, PlanNodeStatus::Ready);
    assert!(retried.branch.is_none());
    assert_eq!(retried.claim_generation, 1);
}

/// Keeps completed and cancelled plans terminal under failed-plan recovery.
#[test]
fn terminal_plans_cannot_use_failed_plan_recovery() {
    for status in [PlanStatus::Completed, PlanStatus::Cancelled] {
        let mut plan = plan_fixture();
        plan.status = status;

        assert_eq!(
            plan.recover_failed_plan(),
            Err(PlanError::InvalidTransition)
        );
        assert_eq!(plan.status, status);
    }
}
