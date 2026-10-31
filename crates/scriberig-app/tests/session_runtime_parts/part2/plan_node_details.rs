//! Verifies bounded gate and evidence reads through application authorization.

use super::*;
use scriberig_session::{
    Branch, Plan, PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner,
    PlanEvidenceReference, PlanEvidenceRole, PlanGate, PlanGatePhase, PlanGateRequirement,
    PlanGateWaiver, PlanNode, PlanNodeParent, PlanNodeStatus, PlanSemanticReference, Session,
    SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use std::sync::Arc;

#[expect(
    clippy::too_many_lines,
    reason = "The regression follows one durable detail read through application authorization."
)]
#[test]
/// Reads bounded node details and rejects invalid page limits.
fn application_reads_plan_node_details_with_limit_validation() {
    let session_id = EntityId::from_bytes([181; 16]);
    let root_id = EntityId::from_bytes([182; 16]);
    let plan_id = EntityId::from_bytes([183; 16]);
    let node_id = EntityId::from_bytes([184; 16]);
    let gate_id = EntityId::from_bytes([185; 16]);
    let evidence_id = EntityId::from_bytes([186; 16]);
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "read gate details",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_id)),
        title: "inspect gates".to_owned(),
        purpose: "read exact gate detail".to_owned(),
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
    })
    .expect("node");
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: node_id,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::Artifact,
        },
        waiver: Some(PlanGateWaiver {
            reason: "explicit fixture waiver".to_owned(),
            actor: None,
            operation: None,
        }),
    })
    .expect("gate");
    plan.attach_evidence(PlanEvidenceAssociation {
        id: evidence_id,
        owner: PlanEvidenceOwner::Gate(gate_id),
        role: PlanEvidenceRole::Informational,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::Artifact,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([187; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: None,
        actor: None,
    })
    .expect("evidence");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan ready");
    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("store"));
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("persist plan");
    let snapshot = store
        .load(session_id)
        .expect("load session")
        .expect("session")
        .plans[0]
        .source_snapshot
        .expect("semantic snapshot");
    let service = ApplicationService::new(Fixture).with_session_store(store);
    let request = scriberig_app::PlanNodeDetailsRequest {
        session: session_id,
        plan: plan_id,
        node: node_id,
        snapshot,
        offset: 0,
        limit: 10,
    };

    let details = service
        .read_plan_node_details(&session_context(), &request)
        .expect("read detail page");
    assert_eq!(details.gates.len(), 1);
    assert_eq!(details.gates[0].id, gate_id);
    assert_eq!(
        details.gates[0].waiver.as_ref().unwrap().reason,
        "explicit fixture waiver"
    );
    assert_eq!(details.evidence.len(), 1);
    assert_eq!(details.evidence[0].id, evidence_id);
    assert!(matches!(
        service.read_plan_node_details(
            &session_context(),
            &scriberig_app::PlanNodeDetailsRequest {
                limit: 257,
                ..request
            }
        ),
        Err(ApplicationError::InvalidRequest { .. })
    ));
}
