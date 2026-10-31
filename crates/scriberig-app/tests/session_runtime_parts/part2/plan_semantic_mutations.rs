//! Verifies shared application mutations for plan summaries, evidence, and gates.

use super::*;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The test verifies the complete semantic mutation flow against exact revisions."
)]
/// Persists exact summary sources, gate evidence, and waiver provenance.
fn application_semantic_plan_mutations_preserve_exact_evidence_and_waivers() {
    let service = ApplicationService::new(Fixture);
    let mut context = session_context();
    let actor = EntityId::from_bytes([161; 16]);
    context.principal = Some(scriberig_app::Principal::new(actor.to_string()));
    let session = EntityId::from_bytes([162; 16]);
    service
        .start_agent(&context, session, "semantic plan mutations")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("session");
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: state.branches[0].id,
                },
                goal: "record evidence and requirements".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "work".to_owned(),
                    parent_index: None,
                    title: "Work".to_owned(),
                    purpose: "complete verified work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    let node = plan.nodes[0].id;
    let source = scriberig_session::PlanSemanticReference {
        entity: EntityId::from_bytes([163; 16]),
        revision: Some(scriberig_core::RevisionId::new([164; 32])),
        snapshot: Some(scriberig_core::SnapshotId::new([165; 32])),
    };
    let summarized = service
        .mutate_plan(
            &context,
            &scriberig_app::PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: scriberig_app::PlanMutation::UpdateSummary {
                    node,
                    text: "The work has exact source evidence.".to_owned(),
                    sources: vec![source],
                },
            },
        )
        .expect("store summary");
    assert_eq!(summarized.summaries[0].sources, vec![source]);
    assert_eq!(summarized.nodes[0].purpose, "complete verified work");

    let gate_added = service
        .mutate_plan(
            &context,
            &scriberig_app::PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: summarized.revision,
                mutation: scriberig_app::PlanMutation::AddGate {
                    owner: node,
                    phase: scriberig_session::PlanGatePhase::Entry,
                    requirement: scriberig_session::PlanGateRequirement::EvidencePresent {
                        kind: scriberig_session::PlanEvidenceKind::BranchResult,
                    },
                },
            },
        )
        .expect("add gate");
    let gate = gate_added.gates[0].id;
    assert_eq!(
        gate_added.evaluate_gate(gate).expect("pending gate").status,
        scriberig_session::PlanGateStatus::Pending
    );
    let evidence = scriberig_session::PlanEvidenceReference {
        kind: scriberig_session::PlanEvidenceKind::BranchResult,
        reference: source,
        attributes: std::collections::BTreeMap::default(),
    };
    let satisfied = service
        .mutate_plan(
            &context,
            &scriberig_app::PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: gate_added.revision,
                mutation: scriberig_app::PlanMutation::AttachEvidence {
                    owner: scriberig_session::PlanEvidenceOwner::Gate(gate),
                    role: scriberig_session::PlanEvidenceRole::Supports,
                    evidence: evidence.clone(),
                    note: Some("verified result".to_owned()),
                },
            },
        )
        .expect("attach supporting evidence");
    let evaluation = satisfied.evaluate_gate(gate).expect("satisfied gate");
    assert_eq!(
        evaluation.status,
        scriberig_session::PlanGateStatus::Satisfied
    );
    assert_eq!(evaluation.supporting_evidence, vec![evidence]);

    let plan_with_waivable_gate = service
        .mutate_plan(
            &context,
            &scriberig_app::PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: satisfied.revision,
                mutation: scriberig_app::PlanMutation::AddGate {
                    owner: node,
                    phase: scriberig_session::PlanGatePhase::Completion,
                    requirement: scriberig_session::PlanGateRequirement::DecisionResolved {
                        decision: EntityId::from_bytes([166; 16]),
                    },
                },
            },
        )
        .expect("add waivable gate");
    let waiver_gate = plan_with_waivable_gate.gates[1].id;
    let waived = service
        .mutate_plan(
            &context,
            &scriberig_app::PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan_with_waivable_gate.revision,
                mutation: scriberig_app::PlanMutation::WaiveGate {
                    gate: waiver_gate,
                    reason: "approved after review".to_owned(),
                },
            },
        )
        .expect("waive gate");
    let provenance = waived
        .gates
        .iter()
        .find(|candidate| candidate.id == waiver_gate)
        .and_then(|candidate| candidate.waiver.as_ref())
        .expect("waiver provenance");
    assert_eq!(provenance.actor, Some(actor));
    assert_eq!(provenance.reason, "approved after review");
    assert_eq!(waived.evidence.len(), 1);
    assert_eq!(
        waived
            .evaluate_gate(waiver_gate)
            .expect("waived gate")
            .status,
        scriberig_session::PlanGateStatus::Waived
    );
}
