//! Verifies bounded plan gate and evidence reads through the remote client.

use super::*;

/// Reads typed gate and evidence detail pages from the exact plan revision.
#[test]
fn remote_client_reads_bounded_plan_details() {
    let client = scriberig_client::Client::new(scriberig_server::InProcessTransport::new(
        ApplicationService::new(Fixture),
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
                    kind: scriberig_protocol::SessionPlanEvidenceKind::Artifact,
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
                    kind: scriberig_protocol::SessionPlanEvidenceKind::Artifact,
                    reference: scriberig_protocol::SessionSemanticReference {
                        entity: root.id,
                        revision: None,
                        snapshot: None,
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
}
