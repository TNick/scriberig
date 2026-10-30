//! Verifies provider-neutral protocol contracts.
use super::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Verifies a value survives JSON encoding without semantic loss.
fn round_trip<T>(value: &T)
where
    T: DeserializeOwned + std::fmt::Debug + PartialEq + Serialize,
{
    let encoded = serde_json::to_string(value).expect("serialize protocol value");
    let decoded: T = serde_json::from_str(&encoded).expect("deserialize protocol value");
    assert_eq!(decoded, *value);
}

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
    let branch = session
        .branches
        .iter()
        .find(|branch| branch.id == "branch-worker-1")
        .expect("worker branch");
    let plan = session.plans.first().expect("plan fixture");
    let dependency = plan.dependencies.first().expect("dependency fixture");

    assert_eq!(
        branch.result.as_ref().map(|result| result.id.as_str()),
        Some("branch-result-1")
    );
    assert_eq!(
        branch
            .result
            .as_ref()
            .map(|result| result.result_handles.as_slice()),
        Some(["validation-1".to_owned()].as_slice())
    );
    assert_eq!(dependency.fulfilled_by.as_deref(), Some("branch-result-1"));
    assert!(session.branches[0].activity.iter().any(|activity| {
        activity.kind == SessionActivityKind::SemanticOperation
            && activity.durable_ref.as_deref() == Some("migration-parser-1")
    }));
    assert!(session.branches[0].activity.iter().any(|activity| {
        activity.kind == SessionActivityKind::Validation
            && activity.durable_ref.as_deref() == Some("validation-1")
    }));
    assert_eq!(plan.revision, 2);
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

/// Ensures remote agent requests carry no provider credential material.
#[test]
fn remote_agent_requests_never_serialize_provider_credentials() {
    let requests = [
        serde_json::to_value(AgentStartRequest {
            session: SessionReference {
                id: "session-1".to_owned(),
            },
            prompt: "inspect this branch".to_owned(),
            selection: None,
        })
        .expect("serialize start request"),
        serde_json::to_value(AgentContinueRequest {
            session: SessionReference {
                id: "session-1".to_owned(),
            },
            branch: BranchReference {
                id: "branch-1".to_owned(),
            },
            prompt: "continue".to_owned(),
            selection: None,
        })
        .expect("serialize continuation request"),
    ];

    for request in requests {
        let object = request.as_object().expect("typed request object");
        assert!(object.keys().all(|key| {
            !key.to_ascii_lowercase().contains("credential")
                && !key.to_ascii_lowercase().contains("secret")
                && !key.to_ascii_lowercase().contains("api_key")
        }));
        assert!(!request.to_string().contains("sentinel-provider-secret"));
    }
}

/// Verifies session and plan contracts round trip.
/// Verifies session snapshots and durable plans round trip.
fn session_snapshot_contracts_round_trip() {
    round_trip(&SessionReadResponse {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        branches: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection {
            revision: 7,
            provider_id: Some("provider-1".to_owned()),
            model_id: Some("model-1".to_owned()),
            thinking_level: Some("careful".to_owned()),
            history: Vec::new(),
        },
    });
    round_trip(&SessionPlan {
        id: "plan-1".to_owned(),
        revision: 4,
        exact_revision: None,
        snapshot: None,
        goal: "complete work".to_owned(),
        status: SessionPlanStatus::Running,
        anchor: Some(SessionContinuationAnchor::BranchResult {
            branch: "branch-1".to_owned(),
            result: "result-1".to_owned(),
        }),
        root_branch: "branch-1".to_owned(),
        nodes: vec![SessionPlanNode {
            id: "node-1".to_owned(),
            parent: "branch:branch-1".to_owned(),
            title: "inspect".to_owned(),
            purpose: "inspect selected entities".to_owned(),
            status: SessionPlanNodeStatus::Running,
            exact_revision: None,
            effective_status: None,
            contains_ignored: false,
            contains_abandoned: false,
            abandon_reason: None,
            branch: Some("branch-2".to_owned()),
            result: None,
            superseded_by: None,
            expected_result: Some(SessionExpectedResult {
                kind: "semantic-summary".to_owned(),
                requires_handle: true,
            }),
            semantic_scope: vec!["entity:1".to_owned()],
            assignment: Some(SessionModelAssignment {
                provider_id: "provider-safe".to_owned(),
                model_id: "model-safe".to_owned(),
                thinking_level: Some("measured".to_owned()),
            }),
            claim: Some(SessionWorkerClaim {
                id: "claim-1".to_owned(),
                node: "node-1".to_owned(),
                branch: "branch-2".to_owned(),
                owner: "runtime".to_owned(),
                generation: 1,
                assignment: None,
            }),
            summary_id: None,
            summary_preview: None,
            dependency_blockers: Vec::new(),
            entry_gates: SessionPlanGateCounts::default(),
            completion_gates: SessionPlanGateCounts::default(),
            evidence_count: 0,
            can_retry: false,
            last_failure: None,
        }],
        dependencies: vec![SessionPlanDependency {
            prerequisite: "node-0".to_owned(),
            dependent: "node-1".to_owned(),
            fulfilled_by: Some("result-0".to_owned()),
        }],
        root_nodes: vec!["node-1".to_owned()],
        status_counts: SessionPlanStatusCounts::default(),
        gates: Vec::new(),
        evidence: Vec::new(),
        summaries: Vec::new(),
        coordinator: Some("branch-1".to_owned()),
        coordinator_cursor: Some("node-1".to_owned()),
        coordinator_claim: Some(SessionCoordinatorClaim {
            id: "coord-1".to_owned(),
            operation: Some("operation-1".to_owned()),
            owner: "runtime".to_owned(),
            generation: 2,
        }),
        history: vec![SessionPlanRevision {
            revision: 3,
            goal: "complete work".to_owned(),
            status: SessionPlanStatus::Paused,
            nodes: Vec::new(),
            dependencies: Vec::new(),
            summaries: Vec::new(),
            gates: Vec::new(),
            evidence: Vec::new(),
            reason: Some("validation follow-up".to_owned()),
        }],
    });
}

/// Verifies bounded plan-attempt request and response contracts round trip.
#[test]
fn plan_attempts_round_trip() {
    round_trip(&SessionPlanAttemptsRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        plan: "plan-1".to_owned(),
        node: "node-1".to_owned(),
        offset: 4,
        limit: 8,
    });
    round_trip(&SessionPlanAttemptsResponse {
        plan: "plan-1".to_owned(),
        node: "node-1".to_owned(),
        total: 5,
        offset: 4,
        attempts: vec![SessionPlanAttempt {
            branch: "branch-4".to_owned(),
            claim_generation: 4,
            claim: Some("claim-4".to_owned()),
            status: SessionPlanNodeStatus::Failed,
            result: None,
            failure: Some(SessionPlanEvidenceReference {
                kind: SessionPlanEvidenceKind::Failure,
                reference: SessionSemanticReference {
                    entity: "failure-4".to_owned(),
                    revision: None,
                    snapshot: None,
                },
                attributes: std::collections::BTreeMap::new(),
            }),
        }],
        next_offset: None,
    });
}

/// Verifies bounded plan gate and evidence detail contracts round trip.
#[test]
fn plan_details_round_trip() {
    round_trip(&SessionPlanDetailsRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        plan: "plan-1".to_owned(),
        expected_revision: 9,
        snapshot: Some("snapshot-9".to_owned()),
        gate_offset: 4,
        evidence_offset: 8,
        limit: 10,
    });
    round_trip(&SessionPlanDetailsResponse {
        plan: "plan-1".to_owned(),
        revision: 9,
        snapshot: Some("snapshot-9".to_owned()),
        gate_total: 4,
        gate_offset: 4,
        gates: Vec::new(),
        gate_next_offset: None,
        evidence_total: 8,
        evidence_offset: 8,
        evidence: Vec::new(),
        evidence_next_offset: None,
    });
}

/// Verifies compact plan graph request and response contracts round trip.
#[test]
fn plan_view_round_trip() {
    round_trip(&SessionPlanViewRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        plan: "plan-1".to_owned(),
        snapshot: Some("snapshot-1".to_owned()),
    });
    round_trip(&SessionPlanGraphView {
        plan: SessionPlanView {
            id: "plan-1".to_owned(),
            revision: "revision-1".to_owned(),
            snapshot: Some("snapshot-1".to_owned()),
            mutation_revision: 3,
            goal: "goal".to_owned(),
            status: SessionPlanStatus::Running,
            coordinator: None,
            coordinator_cursor: None,
            root_nodes: vec!["node-1".to_owned()],
            node_count: 1,
            status_counts: SessionPlanStatusCounts::default(),
        },
        nodes: Vec::new(),
    });
}

/// Verifies exact-snapshot QGIS project links round trip without native provider types.
#[test]
fn qgis_project_reference_contract_round_trip() {
    round_trip(&QgisProjectRequest {
        project: "project-1".to_owned(),
        path: "maps/project.qgs".to_owned(),
        snapshot: "snapshot-1".to_owned(),
    });
    round_trip(&QgisProjectResponse {
        project: "project-1".to_owned(),
        path: "maps/project.qgs".to_owned(),
        snapshot: "snapshot-1".to_owned(),
        revision: "revision-1".to_owned(),
        id: "qgis-project-1".to_owned(),
        title: Some("Planning".to_owned()),
        crs: None,
        layers: vec![QgisLayerReferenceView {
            id: "reference-1".to_owned(),
            name: "Parcels".to_owned(),
            data_source: "./parcels.gpkg|layername=parcels".to_owned(),
            path: Some("./parcels.gpkg".to_owned()),
            resolved_path: Some("maps/parcels.gpkg".to_owned()),
            source_layer: Some("parcels".to_owned()),
            dataset: Some("dataset-1".to_owned()),
            layer: Some("layer-1".to_owned()),
            resolution_diagnostic: None,
            visible: true,
            renderer: Some("categorized".to_owned()),
            scale_range: None,
        }],
        groups: vec![QgisLayerGroupView {
            id: "group-1".to_owned(),
            name: "Administrative".to_owned(),
            layers: vec!["reference-1".to_owned()],
        }],
    });
}

/// Verifies complete QGIS project order requests and candidate responses serialize.
#[test]
fn qgis_layer_order_contract_round_trip() {
    round_trip(&QgisLayerOrderRequest {
        project: "project-1".to_owned(),
        snapshot: "snapshot-1".to_owned(),
        path: "maps/project.qgs".to_owned(),
        ordered_references: vec!["reference-2".to_owned(), "reference-1".to_owned()],
    });
    round_trip(&QgisLayerOrderResponse {
        candidate: "candidate-1".to_owned(),
        project: "project-1".to_owned(),
        base_snapshot: "snapshot-1".to_owned(),
        path: "maps/project.qgs".to_owned(),
    });
}

#[path = "tests_plan_contracts.rs"]
mod plan_contracts;
#[path = "tests_additional.rs"]
mod tests_additional;
#[path = "tests_models.rs"]
mod tests_models;
use tests_models::model_metadata_snapshot;

#[path = "agent_selection_tests.rs"]
mod agent_selection_tests;
#[path = "reference_tests.rs"]
mod reference_tests;
