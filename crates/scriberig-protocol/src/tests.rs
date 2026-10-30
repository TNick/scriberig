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

/// Verifies stable references and snapshot-aware requests round trip.
#[test]
fn references_and_requests_round_trip() {
    let entity = EntityReference {
        id: "entity-1".to_owned(),
    };
    round_trip(&entity);
    round_trip(&InspectEntityRequest {
        entity,
        snapshot: SnapshotSelector::Current,
        detail: request::DetailLevel::Deep,
    });
    round_trip(&QueryRequest {
        specification: serde_json::json!({"scope": "snapshot"}),
    });
    round_trip(&ListRequest {
        specification: serde_json::json!({"collection": "snapshot.entities"}),
    });
    round_trip(&CommandRequest {
        kind: "apply".to_owned(),
        payload: serde_json::json!({"change": "change-1"}),
    });
    round_trip(&DomainOperationCommand {
        capability: "code.add_function".to_owned(),
        entity: EntityReference {
            id: "entity-1".to_owned(),
        },
        snapshot: SnapshotReference {
            id: "snapshot-1".to_owned(),
        },
        revision: None,
        branch: Some("root_working".to_owned()),
        arguments: [("name".to_owned(), serde_json::json!("greet"))]
            .into_iter()
            .collect(),
    });
    round_trip(&AgentStartRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        prompt: "inspect the selected branch".to_owned(),
        selection: None,
    });
    round_trip(&AgentNewSessionRequest {
        prompt: "create a conversation".to_owned(),
        selection: None,
    });
    round_trip(&Request::AgentNewSession(AgentNewSessionRequest {
        prompt: "create a conversation".to_owned(),
        selection: None,
    }));
    round_trip(&AgentContinueRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        branch: BranchReference {
            id: "branch-1".to_owned(),
        },
        prompt: "continue".to_owned(),
        selection: None,
    });
    round_trip(&AgentContinueAtRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        anchor: SessionContinuationAnchor::AssistantOutput {
            branch: "branch-1".to_owned(),
            turn: "turn-7".to_owned(),
        },
        prompt: "continue from this exact answer".to_owned(),
        selection: None,
    });
    session_plan_contracts_round_trip();
    tests_additional::model_contracts_round_trip();
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
            effective_status: Some(SessionPlanNodeStatus::Running),
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

#[expect(
    clippy::too_many_lines,
    reason = "One integration contract checks each request family in the shared protocol."
)]
/// Verifies session plan mutation requests round trip.
fn session_plan_contracts_round_trip() {
    session_snapshot_contracts_round_trip();
    let session = SessionReference {
        id: "session-1".to_owned(),
    };
    round_trip(&SessionPlanCreateRequest {
        session: session.clone(),
        anchor: SessionContinuationAnchor::BranchHead {
            branch: "branch-1".to_owned(),
        },
        goal: "create".to_owned(),
        nodes: vec![SessionPlanNodeDraft {
            key: "node-a".to_owned(),
            parent_index: None,
            title: "Node A".to_owned(),
            purpose: "perform work".to_owned(),
            expected_result: None,
            semantic_scope: vec!["entity:a".to_owned()],
            assignment: None,
        }],
        dependencies: Vec::new(),
    });
    round_trip(&SessionPlanLifecycleRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 4,
        action: SessionPlanAction::Pause,
    });
    round_trip(&SessionPlanAssignmentRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        node: "node-a".to_owned(),
        expected_revision: 5,
        assignment: Some(SessionModelAssignment {
            provider_id: "provider-safe".to_owned(),
            model_id: "model-safe".to_owned(),
            thinking_level: None,
        }),
    });
    round_trip(&SessionPlanReplanRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 6,
        additions: Vec::new(),
        dependencies: vec![(
            SessionPlanNodeReference::Existing {
                id: "node-a".to_owned(),
            },
            SessionPlanNodeReference::Proposed {
                key: "node-b".to_owned(),
            },
        )],
        removed_dependencies: Vec::new(),
        superseded: Vec::new(),
        assignment_changes: Vec::new(),
        preserved_terminal_nodes: Vec::new(),
        reason: "new information".to_owned(),
    });
    round_trip(&SessionPlanRetryRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        node: "node-a".to_owned(),
        expected_revision: 7,
    });
    round_trip(&SessionPlanAttemptHistoryRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        node: "node-a".to_owned(),
        snapshot: "snapshot-1".to_owned(),
        offset: 10,
        limit: 20,
    });
    round_trip(&SessionPlanAttemptHistoryPage {
        offset: 10,
        attempts: vec![SessionPlanAttempt {
            id: "attempt-1".to_owned(),
            exact_revision: "revision-1".to_owned(),
            node: "node-a".to_owned(),
            branch: "branch-a".to_owned(),
            claim_generation: 2,
            claim: Some("claim-2".to_owned()),
            status: SessionPlanNodeStatus::Failed,
            result: None,
            failure: None,
        }],
    });
    round_trip(&SessionPlanNextStepRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 8,
        title: "follow-up".to_owned(),
        purpose: "continue discovered work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        assignment: None,
    });
    round_trip(&SessionPlanDecomposeRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        parent: "node-parent".to_owned(),
        expected_revision: 9,
        children: vec![SessionPlanChildDraft {
            title: "child".to_owned(),
            purpose: "complete one part".to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            assignment: None,
        }],
        required: true,
    });
    round_trip(&SessionPlanSemanticMutationRequest {
        session,
        plan: "plan-1".to_owned(),
        expected_revision: 10,
        mutation: SessionPlanSemanticMutation::UpdateSummary {
            node: "node-a".to_owned(),
            text: "Current progress".to_owned(),
            sources: vec![SessionSemanticReference {
                entity: "source-a".to_owned(),
                revision: Some("rev_001122".to_owned()),
                snapshot: Some("snap_001122".to_owned()),
            }],
        },
    });
}

#[path = "tests_additional.rs"]
mod tests_additional;
#[path = "tests_models.rs"]
mod tests_models;
#[path = "tests_plan_details.rs"]
mod tests_plan_details;
#[path = "tests_session_fixture.rs"]
mod tests_session_fixture;
use tests_models::model_metadata_snapshot;
