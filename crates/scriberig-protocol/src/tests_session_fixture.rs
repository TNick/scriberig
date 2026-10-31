//! Verifies the shared session fixture preserves protocol plan semantics.

use super::*;

/// Verifies the shared recursive session and dependency fixture round trips.
#[test]
fn shared_agent_session_fixture_preserves_cross_surface_semantics() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/agent_session_plan_conformance.json"
    ))
    .expect("shared semantic fixture");
    let session: SessionReadResponse =
        serde_json::from_value(fixture["session"].clone()).expect("session wire model");
    let preferences: AgentPresentationPreferencesResponse =
        serde_json::from_value(fixture["presentation"].clone())
            .expect("shared preference wire model");
    let operation: Operation =
        serde_json::from_value(fixture["operation"].clone()).expect("operation wire model");
    let worker = session
        .branches
        .iter()
        .find(|branch| branch.id == "branch-worker-1")
        .expect("worker branch");
    let plan = session.plans.first().expect("plan fixture");
    let parser = &plan.nodes[0];
    let tests = &plan.nodes[1];
    let retry = &plan.nodes[2];

    assert_eq!(
        worker.result.as_ref().map(|result| result.id.as_str()),
        Some("branch-result-1")
    );
    assert_eq!(
        worker
            .result
            .as_ref()
            .map(|result| result.result_handles.as_slice()),
        Some(["validation-1".to_owned()].as_slice())
    );
    assert_eq!(
        plan.dependencies[0].fulfilled_by.as_deref(),
        fixture["session"]["plans"][0]["dependencies"][0]["fulfilled_by"].as_str()
    );
    assert_eq!(plan.revision, 2);
    assert_eq!(parser.id, "ent_04040404040404040404040404040404");
    assert_eq!(
        parser.effective_status,
        Some(SessionPlanNodeStatus::Completed)
    );
    assert_eq!(parser.evidence_count, 1);
    assert_eq!(tests.effective_status, Some(SessionPlanNodeStatus::Ready));
    assert_eq!(tests.completion_gates.satisfied, 1);
    assert_eq!(tests.evidence_count, 1);
    assert_eq!(
        tests.summary_id.as_deref(),
        Some("ent_0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c")
    );
    assert_eq!(
        retry
            .last_failure
            .as_ref()
            .map(|failure| failure.reference.entity.as_str()),
        Some("ent_06060606060606060606060606060606")
    );
    assert_eq!(plan.gates.len(), 1);
    assert_eq!(plan.evidence.len(), 2);
    assert_eq!(plan.summaries.len(), 1);
    assert_eq!(
        plan.history.first().map(|revision| revision.revision),
        Some(1)
    );
    assert_eq!(preferences.revision, 1);
    assert_eq!(
        preferences.preferences.reasoning,
        PublicReasoningVisibility::Shown
    );
    let result = operation
        .agent_result
        .as_ref()
        .expect("agent operation result");
    assert_eq!(operation.status, OperationStatus::Completed);
    assert_eq!(result.validation_results, vec!["validation passed"]);
    assert_eq!(result.candidate_changes[0].validation, vec!["validation-1"]);
    round_trip(&session);
    round_trip(&preferences);
    round_trip(&operation);
}
