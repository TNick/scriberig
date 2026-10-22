//! Verifies shared application plan mutation behavior.

use super::*;

/// Verifies typed plan mutations use optimistic revisions and preserve structure.
#[test]
/// These operations share one revision boundary and intentionally assert the
/// full lifecycle from gate creation through stale-request rejection.
#[allow(clippy::too_many_lines)]
fn application_plan_semantic_mutations_share_one_revision_boundary() {
    let service = ApplicationService::new(Fixture).with_upstream_models(session_upstream_models());
    let context = session_context();
    let session = EntityId::from_bytes([112; 16]);
    service
        .start_agent(&context, session, "plan anchor")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("session");
    let mut plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: state.branches[0].id,
                },
                goal: "complete semantic work".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "root".to_owned(),
                    parent_index: None,
                    title: "Root".to_owned(),
                    purpose: "coordinate work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("plan");
    plan = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
            },
        )
        .expect("start plan");
    let observed_revision = plan.revision;
    let parent = plan.nodes[0].id;
    plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::AddNextStep(PlanNodeDraft {
                    key: "next".to_owned(),
                    parent_index: None,
                    title: "Next".to_owned(),
                    purpose: "continue the work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }),
            },
        )
        .expect("next step");
    assert_eq!(plan.revision, observed_revision + 1);
    assert_eq!(plan.nodes.len(), 2);
    plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::AddGate {
                    owner: parent,
                    phase: scriberig_session::PlanGatePhase::Completion,
                    requirement: scriberig_session::PlanGateRequirement::EvidencePresent {
                        kind: scriberig_session::PlanEvidenceKind::BranchResult,
                    },
                },
            },
        )
        .expect("add typed gate");
    let gate = plan.gates[0].id;
    plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::AttachEvidence {
                    owner: scriberig_session::PlanEvidenceOwner::Gate(gate),
                    role: scriberig_session::PlanEvidenceRole::Supports,
                    evidence: scriberig_session::PlanEvidenceReference {
                        kind: scriberig_session::PlanEvidenceKind::BranchResult,
                        reference: scriberig_session::PlanSemanticReference {
                            entity: EntityId::from_bytes([113; 16]),
                            revision: None,
                            snapshot: None,
                        },
                        attributes: std::collections::BTreeMap::new(),
                    },
                    note: Some("verified output".to_owned()),
                },
            },
        )
        .expect("attach exact evidence");
    assert_eq!(
        plan.evaluate_gate(gate).unwrap().status,
        scriberig_session::PlanGateStatus::Satisfied
    );
    plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::UpdateSummary {
                    node: parent,
                    text: "Coordination is underway.".to_owned(),
                    sources: vec![scriberig_session::PlanSemanticReference {
                        entity: EntityId::from_bytes([114; 16]),
                        revision: None,
                        snapshot: None,
                    }],
                },
            },
        )
        .expect("update summary");
    plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::Decompose {
                    parent,
                    children: vec![PlanNodeDraft {
                        key: "child".to_owned(),
                        parent_index: None,
                        title: "Child".to_owned(),
                        purpose: "complete a required subtask".to_owned(),
                        expected_result: None,
                        semantic_scope: Vec::new(),
                        assignment: None,
                    }],
                    required: true,
                },
            },
        )
        .expect("decompose parent");
    assert_eq!(plan.nodes.len(), 3);
    assert_eq!(plan.dependencies.len(), 0);
    assert!(plan.summaries.iter().any(|summary| summary.node == parent));
    assert!(plan.gates.iter().any(|item| {
        matches!(
            item.requirement,
            scriberig_session::PlanGateRequirement::RequiredChildCompleted { .. }
        )
    }));
    let stale_request = service.mutate_plan(
        &context,
        &PlanMutationRequest {
            session,
            plan: plan.id,
            expected_revision: 1,
            mutation: PlanMutation::WaiveGate {
                gate: EntityId::new(),
                reason: "stale request".to_owned(),
            },
        },
    );
    assert!(stale_request.is_err());
    assert_eq!(plan.nodes[0].id, parent);
}
