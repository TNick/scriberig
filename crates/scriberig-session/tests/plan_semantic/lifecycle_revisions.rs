//! Verifies stable semantic identities across durable node lifecycle revisions.

use super::*;

/// Resolves the exact semantic revision for one node in an encoded snapshot.
///
/// # Parameters
/// - `snapshot`: Encoded plan at one lifecycle state.
/// - `node`: Stable plan node identity.
///
/// # Returns
/// The content revision recorded for the node.
fn node_revision(snapshot: &PlanSemanticSnapshot, node: EntityId) -> scriberig_core::RevisionId {
    snapshot
        .objects
        .iter()
        .find(|object| object.entity == node)
        .expect("node object")
        .revision
}

/// Preserves node identity and distinct revisions through retry and completion.
#[test]
fn lifecycle_transitions_keep_identity_and_advance_content_revisions() {
    let session = EntityId::from_bytes([61; 16]);
    let branch = EntityId::from_bytes([62; 16]);
    let node_id = EntityId::from_bytes([63; 16]);
    let gate_id = EntityId::from_bytes([64; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([65; 16]),
        session,
        ContinuationAnchor::BranchHead { branch },
        "node lifecycle revisions",
    )
    .expect("plan");
    plan.add_node(node(63, branch, "validated work"))
        .expect("node");
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: node_id,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::ValidationResult,
        },
        waiver: None,
    })
    .expect("entry gate");
    plan.transition(PlanStatus::Planned).expect("plan work");
    let planned = PlanSemanticSnapshot::from_plan(&plan).expect("planned snapshot");
    plan.start_with_coordinator_claim(EntityId::from_bytes([66; 16]), "coordinator", None)
        .expect("start plan");
    plan.attach_evidence(PlanEvidenceAssociation {
        id: EntityId::from_bytes([67; 16]),
        owner: PlanEvidenceOwner::Gate(gate_id),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::ValidationResult,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([68; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: None,
        actor: None,
    })
    .expect("satisfy entry gate");
    assert_eq!(plan.nodes[0].status, PlanNodeStatus::Ready);
    let ready = PlanSemanticSnapshot::from_plan(&plan).expect("ready snapshot");

    let first_branch = EntityId::from_bytes([69; 16]);
    let first_claim = EntityId::from_bytes([70; 16]);
    plan.start_node(node_id, first_branch, first_claim, "worker", &[])
        .expect("start first attempt");
    let running = PlanSemanticSnapshot::from_plan(&plan).expect("running snapshot");
    plan.fail_node(node_id, first_claim)
        .expect("fail first attempt");
    let failed = PlanSemanticSnapshot::from_plan(&plan).expect("failed snapshot");
    plan.retry_failed_node(node_id, &[])
        .expect("retry failed attempt");
    let retried = PlanSemanticSnapshot::from_plan(&plan).expect("retry snapshot");

    let second_branch = EntityId::from_bytes([71; 16]);
    let second_claim = EntityId::from_bytes([72; 16]);
    plan.start_node(node_id, second_branch, second_claim, "worker", &[])
        .expect("start second attempt");
    let second_running = PlanSemanticSnapshot::from_plan(&plan).expect("second running");
    plan.complete_node(
        node_id,
        second_claim,
        &scriberig_session::BranchResult {
            id: EntityId::from_bytes([73; 16]),
            branch: second_branch,
            status: scriberig_session::BranchStatus::Completed,
            summary: None,
            result_kind: None,
            candidate_context: None,
            references: Vec::new(),
            result_handles: Vec::new(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        },
    )
    .expect("complete second attempt");
    let completed = PlanSemanticSnapshot::from_plan(&plan).expect("completed snapshot");

    let snapshots = [
        planned,
        ready,
        running,
        failed,
        retried,
        second_running,
        completed,
    ];
    let revisions = snapshots
        .iter()
        .map(|snapshot| node_revision(snapshot, node_id))
        .collect::<Vec<_>>();
    assert!(revisions.windows(2).all(|pair| pair[0] != pair[1]));
    assert!(snapshots.iter().all(|snapshot| {
        snapshot
            .objects
            .iter()
            .any(|object| object.entity == node_id)
    }));
    assert_eq!(
        plan.nodes[0].status,
        PlanNodeStatus::Completed,
        "the final lifecycle state is durable completion"
    );
}

/// Keeps parent content revision stable when a descendant status rolls up.
#[test]
fn descendant_rollup_change_does_not_revise_parent_content() {
    let branch = EntityId::from_bytes([74; 16]);
    let parent_id = EntityId::from_bytes([75; 16]);
    let child_id = EntityId::from_bytes([76; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([77; 16]),
        EntityId::from_bytes([78; 16]),
        ContinuationAnchor::BranchHead { branch },
        "descendant rollup revision",
    )
    .expect("plan");
    plan.add_node(node(75, branch, "parent"))
        .expect("parent node");
    let mut child = node(76, branch, "child");
    child.parent = Some(PlanNodeParent::Node(parent_id));
    plan.add_node(child).expect("child node");
    let before = PlanSemanticSnapshot::from_plan(&plan).expect("before snapshot");
    plan.nodes
        .iter_mut()
        .find(|candidate| candidate.id == child_id)
        .expect("child")
        .status = PlanNodeStatus::Failed;
    let after = PlanSemanticSnapshot::from_plan(&plan).expect("after snapshot");

    assert_eq!(
        node_revision(&before, parent_id),
        node_revision(&after, parent_id)
    );
    assert_ne!(
        node_revision(&before, child_id),
        node_revision(&after, child_id)
    );
}
