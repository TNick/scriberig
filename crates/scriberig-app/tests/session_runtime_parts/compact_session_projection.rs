//! Verifies the application boundary uses compact session projections for tree reads.

use super::*;

/// Verifies routine reads omit detail collections while full reads preserve them.
#[test]
fn session_projection_omits_plan_history_gates_and_evidence() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-app-session-projection-{}.sqlite",
        EntityId::new()
    ));
    let store = std::sync::Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(&path)
            .expect("open compact session store"),
    );
    let session_store: std::sync::Arc<dyn scriberig_session::SessionStore> = store.clone();
    let service = ApplicationService::new(Fixture).with_session_store(session_store);
    let context = session_context();
    let session = EntityId::new();
    service
        .start_agent(&context, session, "seed session")
        .expect("create session");
    let initial = SessionBackend::read_session(&service, &context, &SessionRequest { session })
        .expect("read complete session");
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: initial.branches[0].id,
                },
                goal: "compact tree read".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "work".to_owned(),
                    parent_index: None,
                    title: "Work".to_owned(),
                    purpose: "verify projection".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    let plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::AddGate {
                    owner: plan.nodes[0].id,
                    phase: scriberig_session::PlanGatePhase::Entry,
                    requirement: scriberig_session::PlanGateRequirement::EvidencePresent {
                        kind: scriberig_session::PlanEvidenceKind::ValidationResult,
                    },
                },
            },
        )
        .expect("add gate");

    let projection =
        SessionBackend::read_session_projection(&service, &context, &SessionRequest { session })
            .expect("read compact session projection");
    let compact = projection
        .plans
        .iter()
        .find(|value| value.id == plan.id)
        .unwrap();
    assert_eq!(compact.history.len(), 0);
    assert_eq!(compact.gates.len(), 0);
    assert_eq!(compact.evidence.len(), 0);

    let complete = SessionBackend::read_session(&service, &context, &SessionRequest { session })
        .expect("read complete session");
    let detailed = complete
        .plans
        .iter()
        .find(|value| value.id == plan.id)
        .unwrap();
    assert_ne!(detailed.history.len(), 0);
    assert_eq!(detailed.gates.len(), 1);
    drop(service);
    drop(store);
    let _ = std::fs::remove_file(path);
}
