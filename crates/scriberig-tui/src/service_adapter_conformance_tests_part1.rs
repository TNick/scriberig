//! Covers additional focused cases for the parent test module.

use super::*;
use crate::service::TuiModelRequestService;

/// Verifies local and remote session adapters share semantics.
///
/// The fixture deliberately compares the full nested session contract in one
/// place so future adapter drift is visible in a single parity assertion.
#[allow(clippy::too_many_lines)]
#[test]
fn local_and_remote_session_adapters_share_semantics() {
    let expected = session_fixture();
    let local = ApplicationAdapter::new(
        LocalSessionFixture {
            session: expected.clone(),
        },
        Vec::new(),
    )
    .read_session(&ApplicationContext::empty(), expected.id)
    .expect("local session");
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "session-equivalence".to_owned(),
            response: Response::Session(SessionReadResponse {
                session: SessionReference {
                    id: expected.id.to_string(),
                },
                branches: vec![SessionBranch {
                    id: expected.branches[0].id.to_string(),
                    parent: None,
                    children: Vec::new(),
                    title: "main".to_owned(),
                    status: SessionBranchStatus::Active,
                    turns: vec![SessionTurn {
                        id: expected.branches[0].turns[0].id.to_string(),
                        ordinal: expected.branches[0].turns[0].ordinal,
                        has_human_input: true,
                        has_agent_output: true,
                        agent_output: Some("fixture output".to_owned()),
                        tool_invocation: None,
                        tool_result: None,
                    }],
                    activity: Vec::new(),
                    result: None,
                }],
                plans: vec![scriberig_protocol::SessionPlan {
                    id: expected.plans[0].id.to_string(),
                    revision: 2,
                    goal: "inspect fixture semantics".to_owned(),
                    status: scriberig_protocol::SessionPlanStatus::Planned,
                    anchor: Some(scriberig_protocol::SessionContinuationAnchor::BranchHead {
                        branch: expected.branches[0].id.to_string(),
                    }),
                    root_branch: expected.branches[0].id.to_string(),
                    nodes: vec![
                        scriberig_protocol::SessionPlanNode {
                            id: expected.plans[0].nodes[0].id.to_string(),
                            parent: expected.branches[0].id.to_string(),
                            title: "produce".to_owned(),
                            purpose: "produce result".to_owned(),
                            status: scriberig_protocol::SessionPlanNodeStatus::Completed,
                            branch: None,
                            result: Some(
                                expected.plans[0].dependencies[0]
                                    .fulfilled_by
                                    .unwrap()
                                    .to_string(),
                            ),
                            superseded_by: None,
                            expected_result: None,
                            semantic_scope: Vec::new(),
                            assignment: None,
                            claim: None,
                        },
                        scriberig_protocol::SessionPlanNode {
                            id: expected.plans[0].nodes[1].id.to_string(),
                            parent: expected.branches[0].id.to_string(),
                            title: "consume".to_owned(),
                            purpose: "consume result".to_owned(),
                            status: scriberig_protocol::SessionPlanNodeStatus::Ready,
                            branch: None,
                            result: None,
                            superseded_by: None,
                            expected_result: None,
                            semantic_scope: Vec::new(),
                            assignment: None,
                            claim: None,
                        },
                    ],
                    dependencies: vec![scriberig_protocol::SessionPlanDependency {
                        prerequisite: expected.plans[0].nodes[0].id.to_string(),
                        dependent: expected.plans[0].nodes[1].id.to_string(),
                        fulfilled_by: expected.plans[0].dependencies[0]
                            .fulfilled_by
                            .map(|result| result.to_string()),
                    }],
                    coordinator: None,
                    coordinator_cursor: None,
                    coordinator_claim: None,
                    history: Vec::new(),
                }],
                execution: scriberig_protocol::SessionExecutionSelection {
                    revision: 1,
                    model_profile: Some("profile-fixture".to_owned()),
                    model: Some("model-fixture".to_owned()),
                    thinking_level: Some("careful".to_owned()),
                    history: Vec::new(),
                },
            }),
        },
    };
    let remote = RemoteAdapter::new(Client::new(transport))
        .read_session(&ApplicationContext::empty(), expected.id)
        .expect("remote session");

    assert_eq!(local.id, remote.id);
    assert_eq!(local.branches.len(), remote.branches.len());
    assert_eq!(local.branches[0].id, remote.branches[0].id);
    assert_eq!(
        local.branches[0].turns[0].ordinal,
        remote.branches[0].turns[0].ordinal
    );
    assert_eq!(
        local.branches[0].turns[0].agent,
        remote.branches[0].turns[0].agent
    );
    assert_eq!(
        local.branches[0].turns[0].human.is_some(),
        remote.branches[0].turns[0].human.is_some()
    );
    assert_eq!(local.plans[0].id, remote.plans[0].id);
    assert_eq!(local.plans[0].revision, remote.plans[0].revision);
    assert_eq!(local.plans[0].goal, remote.plans[0].goal);
    assert_eq!(local.plans[0].status, remote.plans[0].status);
    assert_eq!(local.plans[0].anchor, remote.plans[0].anchor);
    assert_eq!(
        local.plans[0].dependencies[0].fulfilled_by,
        remote.plans[0].dependencies[0].fulfilled_by
    );
    assert_eq!(
        local.plans[0].nodes[1].status,
        remote.plans[0].nodes[1].status
    );
    assert_eq!(local.execution, remote.execution);
}

/// Verifies embedded and remote adapters expose the same operation lifecycle.
#[test]
fn local_and_remote_operation_adapters_share_lifecycle_semantics() {
    let operation =
        scriberig_core::OperationId::new("operation-conformance").expect("operation id");
    let session = session_fixture();
    let entity = scriberig_core::EntityId::from_bytes([44; 16]);
    let agent_result = scriberig_app::AgentResult {
        session: session.id,
        root_branch: session.branches[0].id,
        final_output: "candidate validated".to_owned(),
        branch_result: scriberig_core::EntityId::from_bytes([45; 16]),
        produced_changes: vec![entity],
        validation_results: vec!["validation passed".to_owned()],
        unresolved_questions: Vec::new(),
        candidate_changes: vec![scriberig_app::CandidateChange {
            migration: Some("migration-conformance".to_owned()),
            entity: Some(entity),
            candidate_snapshot: Some(scriberig_core::SnapshotId::new([46; 32])),
            validation: vec!["validation-conformance".to_owned()],
        }],
    };
    let local_service = scriberig_app::ApplicationService::new(LocalSessionFixture {
        session: session.clone(),
    });
    local_service
        .operations()
        .start(operation.clone(), scriberig_app::OperationKind::Agent)
        .expect("start local operation");
    local_service
        .operations()
        .complete_agent(
            &operation,
            vec!["validation-conformance".to_owned()],
            agent_result,
        )
        .expect("complete local operation");
    let local = crate::service::ApplicationOperationAdapter::new(local_service)
        .operation_status(&ApplicationContext::empty(), &operation)
        .expect("local operation status");

    let remote = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "operation-equivalence".to_owned(),
            response: Response::OperationStatus(scriberig_protocol::OperationStatusResponse {
                operation: scriberig_protocol::Operation {
                    reference: OperationReference {
                        id: operation.to_string(),
                    },
                    kind: scriberig_protocol::operation::OperationKind::Agent,
                    status: scriberig_protocol::OperationStatus::Completed,
                    progress: None,
                    result_snapshot: None,
                    result_handles: vec!["validation-conformance".to_owned()],
                    agent_result: Some(scriberig_protocol::AgentResult {
                        session: SessionReference {
                            id: session.id.to_string(),
                        },
                        root_branch: scriberig_protocol::BranchReference {
                            id: session.branches[0].id.to_string(),
                        },
                        final_output: "candidate validated".to_owned(),
                        branch_result: scriberig_core::EntityId::from_bytes([45; 16]).to_string(),
                        produced_changes: vec![scriberig_protocol::EntityReference {
                            id: entity.to_string(),
                        }],
                        validation_results: vec!["validation passed".to_owned()],
                        unresolved_questions: Vec::new(),
                        candidate_changes: vec![scriberig_protocol::CandidateChangeReference {
                            migration: Some("migration-conformance".to_owned()),
                            entity: Some(scriberig_protocol::EntityReference {
                                id: entity.to_string(),
                            }),
                            candidate_snapshot: Some(scriberig_protocol::SnapshotReference {
                                id: scriberig_core::SnapshotId::new([46; 32]).to_string(),
                            }),
                            validation: vec!["validation-conformance".to_owned()],
                        }],
                    }),
                    error: None,
                },
            }),
        },
    }));
    let remote = remote
        .operation_status(&ApplicationContext::empty(), &operation)
        .expect("remote operation status");

    assert_eq!(local, remote);
}

/// Verifies both service paths enforce the same agent-start capability.
#[test]
fn local_and_remote_agent_capability_denial_share_semantics() {
    let session = EntityId::from_bytes([47; 16]);
    let context = ApplicationContext::empty();
    let local = crate::service::ApplicationOperationAdapter::new(
        scriberig_app::ApplicationService::new(LocalSessionFixture {
            session: session_fixture(),
        }),
    )
    .submit_agent_command(&context, session, None, "start")
    .expect_err("local caller without agent_start capability");
    let remote = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "capability-denial".to_owned(),
            response: Response::Error(scriberig_protocol::ProtocolError {
                code: scriberig_protocol::ProtocolErrorCode::PermissionDenied,
                message: "permission denied".to_owned(),
                details: None,
                correlation_id: Some("capability-denial".to_owned()),
            }),
        },
    }))
    .submit_agent_command(&context, session, None, "start")
    .expect_err("remote caller without agent_start capability");

    assert_eq!(local.category(), "permission_denied");
    assert_eq!(remote.category(), local.category());
}

/// Verifies embedded and remote TUI adapters share presentation preferences.
#[test]
fn local_and_remote_presentation_preferences_share_semantics() {
    let context = ApplicationContext::empty();
    let local = ApplicationAdapter::new(
        LocalSessionFixture {
            session: session_fixture(),
        },
        Vec::new(),
    )
    .agent_presentation_preferences(&context)
    .expect("local presentation preferences");
    let remote = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "preference-equivalence".to_owned(),
            response: Response::AgentPresentationPreferences(
                scriberig_protocol::AgentPresentationPreferencesResponse {
                    revision: 4,
                    preferences: scriberig_protocol::PublicAgentPresentationPreferences {
                        tool_output: scriberig_protocol::PublicToolOutputVisibility::Full,
                        ..scriberig_protocol::PublicAgentPresentationPreferences::default()
                    },
                },
            ),
        },
    }))
    .agent_presentation_preferences(&context)
    .expect("remote presentation preferences");

    assert_eq!(local, remote);
}

#[path = "service_adapter_conformance_tests_part1/remote.rs"]
mod remote;
