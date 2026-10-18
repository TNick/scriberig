//! Verifies remote TUI protocol mapping and adapter behavior.

use super::super::service_remote_mapping::{map_remote_operation, remote_error};
use super::RemoteAdapter;
use crate::service::TuiService;
use scriberig_app::ApplicationContext;
use scriberig_client::Client;
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_protocol::{
    AgentAcceptedResponse, CapabilitiesResponse, CapabilityInfo, EntityReference,
    InspectEntityResponse, Operation, OperationStatusResponse, ProtocolTransport, ProtocolVersion,
    RequestEnvelope, Response, ResponseEnvelope, RevisionReference, RuntimeFeature,
    SessionActivityItem, SessionActivityKind, SessionActivityState, SessionBranch,
    SessionBranchStatus, SessionReadResponse, SessionReference, SessionTurn, SnapshotReference,
};

#[path = "service_remote_reconnect_tests.rs"]
mod reconnect_tests;
#[path = "service_remote_session_event_tests.rs"]
mod session_event_tests;

/// Returns one deterministic inspect response without network access.
#[derive(Clone)]
struct FixtureTransport {
    response: ResponseEnvelope,
}

impl ProtocolTransport for FixtureTransport {
    /// Returns the fixture response for every protocol request.
    ///
    /// # Parameters
    /// - `self`: Fixture transport.
    /// - `_request`: Protocol request received from the client.
    ///
    /// # Returns
    /// The deterministic response envelope.
    fn send(&self, _request: RequestEnvelope) -> ResponseEnvelope {
        self.response.clone()
    }
}

/// Verifies remote inspection projects the same stable identities as local inspection.
#[test]
fn remote_inspection_preserves_semantic_handles() {
    let entity = EntityId::from_bytes([1; 16]);
    let revision = RevisionId::new([2; 32]);
    let snapshot = SnapshotId::new([3; 32]);
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "test".to_owned(),
            response: Response::Inspect(InspectEntityResponse {
                entity: EntityReference {
                    id: entity.to_string(),
                },
                revision: RevisionReference {
                    entity: EntityReference {
                        id: entity.to_string(),
                    },
                    id: revision.to_string(),
                },
                snapshot: SnapshotReference {
                    id: snapshot.to_string(),
                },
                object_type: Some("markdown.document".to_owned()),
                summary: Some("fixture".to_owned()),
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let result = adapter
        .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
        .expect("remote inspection");
    assert_eq!(result.entity, entity);
    assert_eq!(result.revision, Some(revision));
    assert_eq!(result.snapshot, Some(snapshot));
}

/// Verifies transport and semantic remote failures retain distinct categories.
#[test]
fn remote_errors_remain_categorized() {
    let transport = remote_error(scriberig_client::ClientError::Transport(
        "connection refused".to_owned(),
    ));
    assert_eq!(transport.category(), "transport_unavailable");
    let semantic = remote_error(scriberig_client::ClientError::Protocol(
        scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::NotFound,
            message: "missing".to_owned(),
            details: None,
            correlation_id: None,
        },
    ));
    assert_eq!(semantic.category(), "not_found");
}

/// Verifies remote agent submission returns the shared operation identity.
#[test]
fn remote_agent_submission_uses_shared_operation() {
    let session = EntityId::from_bytes([4; 16]);
    let branch = EntityId::from_bytes([5; 16]);
    let operation = Operation {
        reference: scriberig_protocol::OperationReference {
            id: "op_agent_remote_1".to_owned(),
        },
        kind: scriberig_protocol::operation::OperationKind::Agent,
        status: scriberig_protocol::OperationStatus::Running,
        progress: None,
        result_snapshot: None,
        result_handles: Vec::new(),
        agent_result: None,
        error: None,
    };
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "agent".to_owned(),
            response: Response::AgentAccepted(AgentAcceptedResponse {
                operation: OperationStatusResponse { operation },
                session: SessionReference {
                    id: session.to_string(),
                },
                branch: Some(scriberig_protocol::BranchReference {
                    id: branch.to_string(),
                }),
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let result = adapter
        .submit_agent_command(&ApplicationContext::empty(), session, None, "inspect")
        .expect("agent operation");
    assert_eq!(result.to_string(), "op_agent_remote_1");
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "agent-handle".to_owned(),
            response: Response::AgentAccepted(AgentAcceptedResponse {
                operation: OperationStatusResponse {
                    operation: Operation {
                        reference: scriberig_protocol::OperationReference {
                            id: "op_agent_remote_2".to_owned(),
                        },
                        kind: scriberig_protocol::operation::OperationKind::Agent,
                        status: scriberig_protocol::OperationStatus::Running,
                        progress: None,
                        result_snapshot: None,
                        result_handles: Vec::new(),
                        agent_result: None,
                        error: None,
                    },
                },
                session: SessionReference {
                    id: session.to_string(),
                },
                branch: Some(scriberig_protocol::BranchReference {
                    id: branch.to_string(),
                }),
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let handle = adapter
        .submit_agent_handle(&ApplicationContext::empty(), session, None, "inspect")
        .expect("agent handles");
    assert_eq!(handle.operation.to_string(), "op_agent_remote_2");
    assert_eq!(handle.session, session);
    assert_eq!(handle.branch, Some(branch));
}

/// Verifies completed remote agent results remain available to TUI operations.
#[test]
fn remote_operation_preserves_agent_result_handles() {
    let session = EntityId::from_bytes([8; 16]);
    let branch = EntityId::from_bytes([9; 16]);
    let branch_result = EntityId::from_bytes([10; 16]);
    let entity = EntityId::from_bytes([11; 16]);
    let snapshot = SnapshotId::new([12; 32]);
    let operation = Operation {
        reference: scriberig_protocol::OperationReference {
            id: "op_agent_result".to_owned(),
        },
        kind: scriberig_protocol::operation::OperationKind::Agent,
        status: scriberig_protocol::OperationStatus::Completed,
        progress: None,
        result_snapshot: None,
        result_handles: vec![branch_result.to_string()],
        agent_result: Some(scriberig_protocol::AgentResult {
            session: SessionReference {
                id: session.to_string(),
            },
            root_branch: scriberig_protocol::BranchReference {
                id: branch.to_string(),
            },
            final_output: "completed".to_owned(),
            branch_result: branch_result.to_string(),
            produced_changes: vec![EntityReference {
                id: entity.to_string(),
            }],
            validation_results: vec!["passed".to_owned()],
            unresolved_questions: Vec::new(),
            candidate_changes: vec![scriberig_protocol::CandidateChangeReference {
                migration: Some("migration-1".to_owned()),
                entity: Some(EntityReference {
                    id: entity.to_string(),
                }),
                candidate_snapshot: Some(SnapshotReference {
                    id: snapshot.to_string(),
                }),
                validation: vec!["validation-1".to_owned()],
            }],
        }),
        error: None,
    };

    let record = map_remote_operation(operation).expect("mapped operation");
    let agent_result = record.agent_result.expect("agent result");
    assert_eq!(agent_result.session, session);
    assert_eq!(agent_result.root_branch, branch);
    assert_eq!(agent_result.branch_result, branch_result);
    assert_eq!(agent_result.produced_changes, vec![entity]);
    assert_eq!(
        agent_result.candidate_changes[0].candidate_snapshot,
        Some(snapshot)
    );
}

/// Verifies remote session reads reconstruct ordered branch activity.
#[test]
fn remote_session_read_reconstructs_branch_activity() {
    let session = EntityId::from_bytes([5; 16]);
    let branch = EntityId::from_bytes([6; 16]);
    let turn = EntityId::from_bytes([7; 16]);
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "session".to_owned(),
            response: Response::Session(SessionReadResponse {
                session: SessionReference {
                    id: session.to_string(),
                },
                branches: vec![SessionBranch {
                    id: branch.to_string(),
                    parent: None,
                    children: Vec::new(),
                    title: "main".to_owned(),
                    status: SessionBranchStatus::Active,
                    turns: vec![SessionTurn {
                        id: turn.to_string(),
                        ordinal: 3,
                        has_human_input: true,
                        has_agent_output: true,
                        agent_output: Some("fixture output".to_owned()),
                        tool_invocation: None,
                        tool_result: None,
                    }],
                    activity: vec![
                        SessionActivityItem {
                            id: format!("{turn}:human"),
                            session: session.to_string(),
                            sequence: 0,
                            branch: branch.to_string(),
                            parent: None,
                            kind: SessionActivityKind::HumanInput,
                            state: SessionActivityState::Completed,
                            summary: "Human input".to_owned(),
                            durable_ref: Some(turn.to_string()),
                            model: None,
                            thinking_level: None,
                            tool_name: None,
                            tool_arguments: None,
                            tool_result_preview: None,
                        },
                        SessionActivityItem {
                            id: format!("{turn}:assistant"),
                            session: session.to_string(),
                            sequence: 1,
                            branch: branch.to_string(),
                            parent: Some(format!("{turn}:human")),
                            kind: SessionActivityKind::AssistantOutput,
                            state: SessionActivityState::Completed,
                            summary: "fixture output".to_owned(),
                            durable_ref: Some(turn.to_string()),
                            model: None,
                            thinking_level: None,
                            tool_name: None,
                            tool_arguments: None,
                            tool_result_preview: None,
                        },
                    ],
                    result: None,
                }],
                execution: scriberig_protocol::SessionExecutionSelection {
                    revision: 4,
                    model_profile: Some("profile-a".to_owned()),
                    model: Some("model-a".to_owned()),
                    thinking_level: Some("careful".to_owned()),
                },
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let value = adapter
        .read_session(&ApplicationContext::empty(), session)
        .expect("remote session");
    assert_eq!(value.id, session);
    assert_eq!(value.branches.len(), 1);
    assert_eq!(value.branches[0].id, branch);
    assert_eq!(value.branches[0].turns[0].id, turn);
    assert_eq!(value.branches[0].turns[0].ordinal, 3);
    assert_eq!(value.execution.revision, 4);
    assert_eq!(value.execution.model.as_deref(), Some("model-a"));
    assert_eq!(value.execution.thinking_level.as_deref(), Some("careful"));
    assert!(value.branches[0].turns[0].human.is_some());
    assert!(value.branches[0].turns[0].agent.is_some());
    assert_eq!(
        value.branches[0].turns[0].agent.as_ref().unwrap().text,
        "fixture output"
    );
    assert_eq!(
        value.branches[0].activity,
        vec![
            scriberig_session::ExecutionActivity::HumanInput(turn),
            scriberig_session::ExecutionActivity::AgentOutput(turn),
        ]
    );
}

/// Verifies remote cancellation uses the shared operation lifecycle endpoint.
#[test]
fn remote_cancellation_returns_shared_lifecycle_state() {
    let operation = Operation {
        reference: scriberig_protocol::OperationReference {
            id: "op_cancel_remote_1".to_owned(),
        },
        kind: scriberig_protocol::operation::OperationKind::Agent,
        status: scriberig_protocol::OperationStatus::Cancelled,
        progress: None,
        result_snapshot: None,
        result_handles: Vec::new(),
        agent_result: None,
        error: None,
    };
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "cancel".to_owned(),
            response: Response::OperationCancelled(OperationStatusResponse { operation }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let record = adapter
        .cancel_operation(
            &ApplicationContext::empty(),
            &scriberig_core::OperationId::new("op_cancel_remote_1").expect("operation identity"),
        )
        .expect("remote cancellation");
    assert_eq!(record.status, scriberig_app::OperationStatus::Cancelled);
    assert_eq!(record.kind, scriberig_app::OperationKind::Agent);
}

/// Verifies remote capability discovery reaches the TUI action availability model.
#[test]
fn remote_capabilities_are_decoded_without_provider_details() {
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "capabilities".to_owned(),
            response: Response::Capabilities(CapabilitiesResponse {
                protocol: ProtocolVersion::default(),
                capabilities: vec![CapabilityInfo {
                    id: "agent_start".to_owned(),
                    description: "Start an agent operation".to_owned(),
                    available: true,
                    domain: Some("session".to_owned()),
                }],
                features: vec![RuntimeFeature {
                    id: "agent".to_owned(),
                    enabled: true,
                }],
            }),
        },
    };
    let adapter = RemoteAdapter::new(Client::new(transport));
    let capabilities = adapter
        .runtime_capabilities(&ApplicationContext::empty())
        .expect("remote capabilities");
    assert_eq!(capabilities.len(), 1);
    assert_eq!(capabilities[0].id.to_string(), "agent_start");
    assert!(capabilities[0].available);
    assert_eq!(capabilities[0].scope.as_deref(), Some("session"));
    let palette = crate::overlay::palette::PaletteState::with_capabilities(&capabilities);
    assert!(
        palette
            .local_actions
            .iter()
            .any(|item| item.id == "start-agent")
    );
    assert!(
        !palette
            .local_actions
            .iter()
            .any(|item| item.id == "continue-agent")
    );
}

#[path = "service_adapter_conformance_tests.rs"]
mod adapter_conformance_tests;
