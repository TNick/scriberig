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
    model_contracts_round_trip();
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
        }],
        dependencies: vec![SessionPlanDependency {
            prerequisite: "node-0".to_owned(),
            dependent: "node-1".to_owned(),
            fulfilled_by: Some("result-0".to_owned()),
        }],
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
            reason: Some("validation follow-up".to_owned()),
        }],
    });
}

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
        session,
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
}

/// Verifies model, prompt, and presentation contracts round trip.
fn model_contracts_round_trip() {
    round_trip(&ModelRequestDetailRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        branch: BranchReference {
            id: "branch-1".to_owned(),
        },
        invocation: "invocation-1".to_owned(),
    });
    round_trip(&ModelRequestDetailResponse {
        invocation: "invocation-1".to_owned(),
        model: "model-1".to_owned(),
        thinking_level: Some("careful".to_owned()),
        messages: vec![ModelRequestMessageDetail {
            role: "user".to_owned(),
            content: "safe prompt".to_owned(),
        }],
        tools: vec![ModelRequestToolDetail {
            name: "inspect".to_owned(),
            description: "Inspect an entity".to_owned(),
            parameters: Some("{}".to_owned()),
        }],
        truncated: false,
    });
    round_trip(&ModelLibraryRequest::default());
    round_trip(&SessionModelSelectionRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        expected_revision: 7,
        provider_id: "provider-1".to_owned(),
        model_id: "model-1".to_owned(),
    });
    round_trip(&SessionThinkingSelectionRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        expected_revision: 8,
        thinking_level: "extended".to_owned(),
    });
    round_trip(&ModelLibraryResponse {
        models: vec![PublicModelDescriptor {
            id: "model-1".to_owned(),
            display_name: "Safe Model Name".to_owned(),
            provider_profile: "provider-1".to_owned(),
            provider_model_id: "provider-model-1".to_owned(),
            capabilities: PublicModelCapabilities {
                streaming: true,
                tool_calls: true,
                structured_output: false,
            },
            thinking_levels: vec![PublicThinkingLevelDescriptor {
                id: "deep-dive".to_owned(),
                label: "Deep dive".to_owned(),
                description: None,
                order: 0,
            }],
            default_thinking_level: Some("deep-dive".to_owned()),
        }],
        profiles: vec![PublicModelProfile {
            id: "profile-1".to_owned(),
            display_name: "Primary".to_owned(),
            model: "model-1".to_owned(),
            connection: "connection-1".to_owned(),
            credential: Some(ModelCredentialRef {
                id: "secret-ref-1".to_owned(),
            }),
            default_thinking_level: Some("deep-dive".to_owned()),
            enabled: true,
        }],
    });
}

#[path = "tests_additional.rs"]
mod tests_additional;
