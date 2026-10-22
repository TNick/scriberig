//! Verifies shared application plan mutation behavior.

use super::*;

/// Verifies compact views preserve gate and evidence summaries from SQLite.
#[test]
fn compact_plan_view_reads_cached_gate_summaries() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-compact-plan-view-{}.sqlite",
        EntityId::new()
    ));
    let store = std::sync::Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(&path).expect("open compact plan store"),
    );
    let session_store: std::sync::Arc<dyn scriberig_session::SessionStore> = store.clone();
    let service = ApplicationService::new(Fixture).with_session_store(session_store);
    let context = session_context();
    let session = EntityId::from_bytes([132; 16]);
    service
        .start_agent(&context, session, "compact plan view")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("read session");
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: state.branches[0].id,
                },
                goal: "inspect compactly".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "root".to_owned(),
                    parent_index: None,
                    title: "Root".to_owned(),
                    purpose: "verify compact projection".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    let owner = plan.nodes[0].id;
    let plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::AddGate {
                    owner,
                    phase: scriberig_session::PlanGatePhase::Entry,
                    requirement: scriberig_session::PlanGateRequirement::EvidencePresent {
                        kind: scriberig_session::PlanEvidenceKind::BranchResult,
                    },
                },
            },
        )
        .expect("add entry gate");
    let gate = plan.gates[0].id;
    let plan = service
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
                            entity: EntityId::from_bytes([133; 16]),
                            revision: None,
                            snapshot: None,
                        },
                        attributes: std::collections::BTreeMap::new(),
                    },
                    note: None,
                },
            },
        )
        .expect("attach supporting evidence");

    let (view, nodes) = service
        .read_plan_views(&context, session, plan.id, None)
        .expect("read compact plan views");

    assert_eq!(view.node_count, 1);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, owner);
    assert_eq!(nodes[0].entry_gates.satisfied, 1);
    assert_eq!(nodes[0].entry_gates.pending, 0);
    assert_eq!(nodes[0].evidence_count, 1);
    drop(service);
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Verifies typed plan mutations use optimistic revisions and preserve structure.
#[test]
/// These operations share one revision boundary and intentionally assert the
/// full lifecycle from gate creation through stale-request rejection.
#[allow(clippy::too_many_lines)]
fn application_plan_semantic_mutations_share_one_revision_boundary() {
    let service = ApplicationService::new(Fixture).with_upstream_models(session_upstream_models());
    let actor = EntityId::from_bytes([115; 16]);
    let mut context = session_context();
    context.principal = Some(scriberig_app::Principal::new(actor.to_string()));
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
    assert_eq!(plan.evidence[0].actor, Some(actor));
    plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::WaiveGate {
                    gate,
                    reason: "approved by the session owner".to_owned(),
                },
            },
        )
        .expect("waive typed gate");
    let waiver = plan.gates[0].waiver.as_ref().expect("inspectable waiver");
    assert_eq!(plan.gates[0].id, gate);
    assert_eq!(waiver.reason, "approved by the session owner");
    assert_eq!(waiver.actor, Some(actor));
    assert_eq!(plan.evidence.len(), 1);
    assert_eq!(
        plan.evaluate_gate(gate).expect("waived gate").status,
        scriberig_session::PlanGateStatus::Waived
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
    let parent_rollup = plan.node_rollups().expect("plan rollups")[&parent];
    assert_eq!(parent_rollup.unsatisfied_completion_gates, 1);
    assert_ne!(
        parent_rollup.effective_status,
        scriberig_session::PlanNodeStatus::Completed
    );
    let child = plan
        .nodes
        .iter()
        .find(|node| node.parent == Some(scriberig_session::PlanNodeParent::Node(parent)))
        .expect("decomposed child")
        .id;
    let parent_summary = plan
        .summaries
        .iter()
        .find(|summary| summary.node == parent)
        .expect("parent summary")
        .text
        .clone();
    plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::UpdateSummary {
                    node: child,
                    text: "The child change has its own summary.".to_owned(),
                    sources: Vec::new(),
                },
            },
        )
        .expect("summarize child separately");
    assert_eq!(
        plan.summaries
            .iter()
            .find(|summary| summary.node == parent)
            .expect("unchanged parent summary")
            .text,
        parent_summary
    );
    assert!(plan.summaries.iter().any(|summary| summary.node == child));
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
