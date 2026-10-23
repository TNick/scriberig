//! Verifies bounded plan gate and evidence reads through the remote client.

use super::*;

#[allow(
    clippy::too_many_lines,
    reason = "the test verifies one remote paging flow end to end"
)]
/// Reads typed gate and evidence detail pages from the exact plan revision.
#[test]
fn remote_client_reads_bounded_plan_details() {
    let store = std::sync::Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open_in_memory().expect("open store"),
    );
    let client = scriberig_client::Client::new(scriberig_server::InProcessTransport::new(
        ApplicationService::new(Fixture).with_session_store(store),
    ));
    let session = "ent_72727272727272727272727272727272";
    let started = client
        .start_agent_with_handles(session, "seed plan details")
        .expect("seed session");
    let root = started.branch.expect("root branch");
    let mut plan = client
        .create_plan(SessionPlanCreateRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            anchor: SessionContinuationAnchor::BranchHead {
                branch: root.id.clone(),
            },
            goal: "bounded requirement details".to_owned(),
            nodes: vec![SessionPlanNodeDraft {
                key: "work".to_owned(),
                parent_index: None,
                title: "Work".to_owned(),
                purpose: "inspect requirements".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            }],
            dependencies: Vec::new(),
        })
        .expect("create plan");
    plan = client
        .mutate_plan(SessionPlanMutationRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: plan.id.clone(),
            expected_revision: plan.revision,
            mutation: SessionPlanMutation::AddGate {
                owner: plan.nodes[0].id.clone(),
                phase: scriberig_protocol::SessionPlanGatePhase::Entry,
                requirement: scriberig_protocol::SessionPlanGateRequirement::EvidencePresent {
                    kind: scriberig_protocol::SessionPlanEvidenceKind::ValidationResult,
                },
            },
        })
        .expect("add gate");
    plan = client
        .mutate_plan(SessionPlanMutationRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: plan.id.clone(),
            expected_revision: plan.revision,
            mutation: SessionPlanMutation::AttachEvidence {
                owner: scriberig_protocol::SessionPlanEvidenceOwner::Gate(plan.gates[0].id.clone()),
                role: scriberig_protocol::SessionPlanEvidenceRole::Supports,
                evidence: scriberig_protocol::SessionPlanEvidenceReference {
                    kind: scriberig_protocol::SessionPlanEvidenceKind::ValidationResult,
                    reference: scriberig_protocol::SessionSemanticReference {
                        entity: root.id,
                        revision: Some(
                            scriberig_core::RevisionId::from_bytes([73; 32]).to_string(),
                        ),
                        snapshot: Some(
                            scriberig_core::SnapshotId::from_bytes([74; 32]).to_string(),
                        ),
                    },
                    attributes: std::collections::BTreeMap::new(),
                },
                note: None,
            },
        })
        .expect("attach evidence");

    let details = client
        .plan_details(scriberig_protocol::SessionPlanDetailsRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: plan.id.clone(),
            expected_revision: plan.revision,
            snapshot: None,
            gate_offset: 0,
            evidence_offset: 0,
            limit: 1,
        })
        .expect("read plan details");

    assert_eq!(details.revision, plan.revision);
    assert_eq!(details.gate_total, 1);
    assert_eq!(details.evidence_total, 1);
    assert_eq!(details.gates[0].id, plan.gates[0].id);
    assert_eq!(details.evidence[0].id, plan.evidence[0].id);
    assert_eq!(
        details.evidence[0].evidence.reference.revision,
        plan.evidence[0].evidence.reference.revision
    );
    assert_eq!(
        details.evidence[0].evidence.reference.snapshot,
        plan.evidence[0].evidence.reference.snapshot
    );

    let historical_snapshot = plan.snapshot.clone().expect("plan snapshot");
    let historical_revision = plan.revision;
    let updated = client
        .mutate_plan(SessionPlanMutationRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: plan.id.clone(),
            expected_revision: plan.revision,
            mutation: SessionPlanMutation::AddGate {
                owner: plan.nodes[0].id.clone(),
                phase: scriberig_protocol::SessionPlanGatePhase::Completion,
                requirement: scriberig_protocol::SessionPlanGateRequirement::EvidencePresent {
                    kind: scriberig_protocol::SessionPlanEvidenceKind::Artifact,
                },
            },
        })
        .expect("add later gate");
    assert_eq!(updated.gates.len(), 2);
    let historical = client
        .plan_details(scriberig_protocol::SessionPlanDetailsRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: plan.id,
            expected_revision: historical_revision,
            snapshot: Some(historical_snapshot.clone()),
            gate_offset: 0,
            evidence_offset: 0,
            limit: 100,
        })
        .expect("read historical plan details");
    assert_eq!(historical.revision, historical_revision);
    assert_eq!(
        historical.snapshot.as_deref(),
        Some(historical_snapshot.as_str())
    );
    assert_eq!(historical.gate_total, 1);
    let compact = client
        .plan_view(scriberig_protocol::SessionPlanViewRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: updated.id,
            snapshot: Some(historical_snapshot.clone()),
        })
        .expect("read compact historical plan view");
    assert_eq!(compact.plan.revision, plan.exact_revision.unwrap());
    assert_eq!(
        compact.plan.snapshot.as_deref(),
        Some(historical_snapshot.as_str())
    );
    assert_eq!(compact.plan.node_count, 1);
    assert_eq!(compact.nodes.len(), 1);
    assert_eq!(
        compact.nodes[0].entry_gates.pending
            + compact.nodes[0].entry_gates.satisfied
            + compact.nodes[0].entry_gates.failed
            + compact.nodes[0].entry_gates.waived,
        1
    );
}
