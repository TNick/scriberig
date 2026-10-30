//! Verifies immutable semantic plan object encoding and assembly.

use scriberig_core::EntityId;
use scriberig_session::{
    ContinuationAnchor, Plan, PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner,
    PlanEvidenceReference, PlanEvidenceRole, PlanGate, PlanGatePhase, PlanGateRequirement,
    PlanGateStatus, PlanGateWaiver, PlanNode, PlanNodeParent, PlanNodeRollupIndex, PlanNodeStatus,
    PlanNodeSummary, PlanSemanticReference, PlanSemanticSnapshot, PlanStatus,
};

/// Creates a valid unstarted plan node with a stable identity.
///
/// # Parameters
/// - `id`: Stable node identity byte repeated across the ID.
/// - `branch`: Existing branch used as the structural parent.
/// - `title`: Human-readable node title and purpose.
///
/// # Returns
/// A planned node with no execution attempt.
fn node(id: u8, branch: EntityId, title: &str) -> PlanNode {
    PlanNode {
        id: EntityId::from_bytes([id; 16]),
        parent: Some(PlanNodeParent::Branch(branch)),
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

/// Round-trips independent gate, evidence, and summary records.
#[test]
fn semantic_plan_requirement_records_round_trip_and_block_entry() {
    let branch = EntityId::from_bytes([2; 16]);
    let node_id = EntityId::from_bytes([4; 16]);
    let gate_id = EntityId::from_bytes([6; 16]);
    let evidence_id = EntityId::from_bytes([7; 16]);
    let summary_id = EntityId::from_bytes([8; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        EntityId::from_bytes([1; 16]),
        ContinuationAnchor::BranchHead { branch },
        "gate round trip",
    )
    .expect("valid plan");
    plan.add_node(node(4, branch, "validated step"))
        .expect("node");
    plan.transition(PlanStatus::Planned).expect("planned");
    plan.transition(PlanStatus::Running).expect("running");
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: node_id,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::ValidationPassed {
            profile: Some("lint".to_owned()),
        },
        waiver: None,
    })
    .expect("gate");

    assert_eq!(plan.nodes[0].status, PlanNodeStatus::Planned);
    assert_eq!(
        plan.evaluate_gate(gate_id).expect("evaluation").status,
        PlanGateStatus::Pending
    );
    assert_eq!(plan.ready_nodes_from_results(&[]), Vec::new());

    let mut attributes = std::collections::BTreeMap::new();
    attributes.insert("passed".to_owned(), "true".to_owned());
    attributes.insert("profile".to_owned(), "lint".to_owned());
    plan.attach_evidence(PlanEvidenceAssociation {
        id: evidence_id,
        owner: PlanEvidenceOwner::Gate(gate_id),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::ValidationResult,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([9; 16]),
                revision: Some(scriberig_core::RevisionId::new([10; 32])),
                snapshot: Some(scriberig_core::SnapshotId::new([11; 32])),
            },
            attributes,
        },
        note: Some("lint passed".to_owned()),
        actor: None,
    })
    .expect("evidence");
    plan.attach_evidence(PlanEvidenceAssociation {
        id: EntityId::from_bytes([13; 16]),
        owner: PlanEvidenceOwner::Node(node_id),
        role: PlanEvidenceRole::Informational,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::BranchResult,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([14; 16]),
                revision: Some(scriberig_core::RevisionId::new([15; 32])),
                snapshot: Some(scriberig_core::SnapshotId::new([16; 32])),
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: None,
        actor: None,
    })
    .expect("branch result evidence");
    plan.update_summary(PlanNodeSummary {
        id: summary_id,
        node: node_id,
        text: "Validation is complete".to_owned(),
        sources: vec![PlanSemanticReference {
            entity: EntityId::from_bytes([9; 16]),
            revision: Some(scriberig_core::RevisionId::new([10; 32])),
            snapshot: Some(scriberig_core::SnapshotId::new([11; 32])),
        }],
    })
    .expect("summary");

    assert_eq!(plan.nodes[0].status, PlanNodeStatus::Ready);
    assert_eq!(plan.ready_nodes_from_results(&[]), vec![node_id]);
    let evaluation = plan.evaluate_gate(gate_id).expect("evaluation");
    assert_eq!(evaluation.status, PlanGateStatus::Satisfied);
    assert_eq!(
        evaluation.supporting_evidence,
        vec![plan.evidence[0].evidence.clone()]
    );
    let snapshot = PlanSemanticSnapshot::from_plan(&plan).expect("encode");
    assert_node_detail_pages(&snapshot, &plan, node_id, gate_id);
    let decoded = snapshot.into_plan(plan.history.clone()).expect("decode");
    assert_eq!(decoded, plan);
    assert_summary_view(&plan, node_id, summary_id);
}

/// Verifies a plan node view retains its summary and work purpose.
///
/// # Parameters
/// - `plan`: Plan used to project the node.
/// - `node_id`: Stable node identity to inspect.
/// - `summary_id`: Stable summary identity expected in the view.
fn assert_summary_view(plan: &Plan, node_id: EntityId, summary_id: EntityId) {
    let rollups = plan.node_rollups().expect("rollups");
    let view = plan
        .node_view(node_id, None, None, &rollups)
        .expect("node view");

    assert_eq!(view.summary_id, Some(summary_id));
    assert_eq!(
        view.summary_preview.as_deref(),
        Some("Validation is complete")
    );
    assert_eq!(view.purpose, "validated step");
    assert_eq!(view.result, None);
}

/// Marks a validation gate failed when exact evidence contradicts it.
#[test]
fn contradictory_validation_evidence_fails_entry_gate() {
    let branch = EntityId::from_bytes([2; 16]);
    let node_id = EntityId::from_bytes([4; 16]);
    let gate_id = EntityId::from_bytes([6; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        EntityId::from_bytes([1; 16]),
        ContinuationAnchor::BranchHead { branch },
        "contradictory gate evidence",
    )
    .expect("plan");
    plan.add_node(node(4, branch, "validated step"))
        .expect("node");
    plan.transition(PlanStatus::Planned).expect("planned");
    plan.transition(PlanStatus::Running).expect("running");
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: node_id,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::ValidationPassed {
            profile: Some("lint".to_owned()),
        },
        waiver: None,
    })
    .expect("gate");
    plan.attach_evidence(PlanEvidenceAssociation {
        id: EntityId::from_bytes([17; 16]),
        owner: PlanEvidenceOwner::Gate(gate_id),
        role: PlanEvidenceRole::Contradicts,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::ValidationResult,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([18; 16]),
                revision: Some(scriberig_core::RevisionId::new([19; 32])),
                snapshot: Some(scriberig_core::SnapshotId::new([20; 32])),
            },
            attributes: std::collections::BTreeMap::from([
                ("passed".to_owned(), "false".to_owned()),
                ("profile".to_owned(), "lint".to_owned()),
            ]),
        },
        note: None,
        actor: None,
    })
    .expect("contradict gate evidence");

    assert_eq!(
        plan.evaluate_gate(gate_id)
            .expect("contradicted gate")
            .status,
        PlanGateStatus::Failed
    );
    assert_eq!(plan.ready_nodes_from_results(&[]), Vec::new());
}

/// Derives effective parent status and ignored/abandoned descendant flags.
#[test]
fn plan_rollup_respects_required_abandoned_child_and_waiver() {
    let mut plan = plan();
    let parent = EntityId::from_bytes([10; 16]);
    let child = EntityId::from_bytes([11; 16]);
    let gate = EntityId::from_bytes([12; 16]);
    let branch = plan.root_branch;
    plan.add_node(node(10, branch, "parent")).expect("parent");
    plan.nodes
        .iter_mut()
        .find(|node| node.id == parent)
        .expect("parent node")
        .status = PlanNodeStatus::Completed;
    let mut child_node = node(11, branch, "ignored child");
    child_node.parent = Some(PlanNodeParent::Node(parent));
    child_node.status = PlanNodeStatus::Abandoned;
    child_node.abandon_reason = Some(scriberig_session::PlanAbandonReason::Ignored);
    plan.add_node(child_node).expect("child");
    plan.add_gate(PlanGate {
        id: gate,
        owner: parent,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::RequiredChildCompleted { child },
        waiver: None,
    })
    .expect("required child gate");

    let rollup = plan.node_rollups().expect("rollups")[&parent];
    assert_eq!(rollup.effective_status, PlanNodeStatus::Planned);
    assert!(rollup.contains_ignored);
    assert!(rollup.contains_abandoned);
    assert_eq!(rollup.unsatisfied_completion_gates, 1);

    plan.waive_gate(
        gate,
        PlanGateWaiver {
            reason: "ignored after review".to_owned(),
            actor: None,
            operation: None,
        },
    )
    .expect("waiver");
    let waived = plan.node_rollups().expect("waived rollups")[&parent];
    assert_eq!(waived.effective_status, PlanNodeStatus::Completed);
    assert_eq!(waived.unsatisfied_completion_gates, 0);
    assert!(waived.contains_ignored);
    assert_eq!(
        plan.evaluate_gate(gate).expect("waived gate").status,
        PlanGateStatus::Waived
    );
}

/// Checks deterministic rollup precedence across required child outcomes.
#[test]
fn plan_rollup_status_cases_are_table_driven() {
    let cases = [
        (PlanNodeStatus::Planned, None, PlanNodeStatus::Planned),
        (PlanNodeStatus::Ready, None, PlanNodeStatus::Ready),
        (PlanNodeStatus::Running, None, PlanNodeStatus::Running),
        (PlanNodeStatus::Failed, None, PlanNodeStatus::Failed),
        (PlanNodeStatus::Completed, None, PlanNodeStatus::Completed),
        (
            PlanNodeStatus::Abandoned,
            Some(scriberig_session::PlanAbandonReason::Ignored),
            PlanNodeStatus::Planned,
        ),
    ];
    let branch = EntityId::from_bytes([2; 16]);

    for (index, (child_status, abandon_reason, expected)) in cases.into_iter().enumerate() {
        let index = u8::try_from(index).expect("rollup case count fits in u8");
        let parent_id = EntityId::from_bytes([100 + index; 16]);
        let child_id = EntityId::from_bytes([120 + index; 16]);
        let mut plan = Plan::new(
            EntityId::from_bytes([140 + index; 16]),
            EntityId::from_bytes([1; 16]),
            ContinuationAnchor::BranchHead { branch },
            "table-driven rollup",
        )
        .expect("plan");
        plan.add_node(node(100 + index, branch, "parent"))
            .expect("parent");
        let mut child = node(120 + index, branch, "child");
        child.parent = Some(PlanNodeParent::Node(parent_id));
        plan.add_node(child).expect("child");
        plan.add_gate(PlanGate {
            id: EntityId::from_bytes([160 + index; 16]),
            owner: parent_id,
            phase: PlanGatePhase::Completion,
            requirement: PlanGateRequirement::RequiredChildCompleted { child: child_id },
            waiver: None,
        })
        .expect("required child gate");
        let child_node = plan
            .nodes
            .iter_mut()
            .find(|candidate| candidate.id == child_id)
            .expect("child node");
        child_node.status = child_status;
        child_node.abandon_reason = abandon_reason;
        plan.nodes
            .iter_mut()
            .find(|candidate| candidate.id == parent_id)
            .expect("parent node")
            .status = PlanNodeStatus::Completed;

        let rollup = plan.node_rollups().expect("rollup")[&parent_id];
        assert_eq!(rollup.effective_status, expected);
        assert_eq!(rollup.contains_ignored, abandon_reason.is_some());
    }
}

/// Rejects malformed structural cycles before deriving effective status.
#[test]
fn plan_rollup_rejects_structural_cycles() {
    let mut plan = plan();
    let first = plan.nodes[0].id;
    let second = plan.nodes[1].id;
    plan.nodes[0].parent = Some(PlanNodeParent::Node(second));
    plan.nodes[1].parent = Some(PlanNodeParent::Node(first));

    assert_eq!(
        plan.node_rollups(),
        Err(scriberig_session::PlanError::StructuralCycle)
    );
}

/// Prevents a malformed structural cycle from reaching semantic persistence.
#[test]
fn semantic_encoder_rejects_structural_cycles() {
    let mut plan = plan();
    let first = plan.nodes[0].id;
    let second = plan.nodes[1].id;
    plan.nodes[0].parent = Some(PlanNodeParent::Node(second));
    plan.nodes[1].parent = Some(PlanNodeParent::Node(first));

    let encoded = PlanSemanticSnapshot::from_plan(&plan);
    assert!(
        matches!(
            &encoded,
            Err(scriberig_session::PlanSemanticError::InvalidObject(_))
        ),
        "{encoded:?}"
    );
    assert!(format!("{encoded:?}").contains("StructuralCycle"));
}

#[path = "plan_semantic_parts/rollup_views.rs"]
mod rollup_views;

#[path = "plan_semantic_parts/node_details.rs"]
mod node_details;

#[path = "plan_semantic_parts/snapshots.rs"]
mod snapshots;

use node_details::assert_node_detail_pages;
use snapshots::plan;
