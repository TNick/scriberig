//! Verifies remote TUI protocol mapping and adapter behavior.

use super::super::service_remote_mapping::{
    map_remote_operation, remote_error, session_from_response,
};
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

#[path = "service_remote_plan_tests.rs"]
mod plan_tests;
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
    /// - `self`: Fixture transport.
    /// - `_request`: Protocol request received from the client.
    ///
    /// The deterministic response envelope.
    /// # Returns
    /// The operation result.
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
    let local_denial = scriberig_app::ApplicationError::PermissionDenied {
        action: "session.continue".to_owned(),
        scope: "session-1".to_owned(),
    };
    let remote_denial = remote_error(scriberig_client::ClientError::Protocol(Box::new(
        scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::PermissionDenied,
            message: "permission denied".to_owned(),
            details: None,
            correlation_id: None,
        },
    )));
    assert_eq!(local_denial.category(), "permission_denied");
    assert_eq!(remote_denial.category(), local_denial.category());

    let transport = remote_error(scriberig_client::ClientError::Transport(
        "connection refused".to_owned(),
    ));
    assert_eq!(transport.category(), "transport_unavailable");
    let semantic = remote_error(scriberig_client::ClientError::Protocol(Box::new(
        scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::NotFound,
            message: "missing".to_owned(),
            details: None,
            correlation_id: None,
        },
    )));
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

/// Uses remote new-session acceptance handles without local identity guesses.
#[test]
fn remote_new_session_handle_preserves_authoritative_identities() {
    let session = EntityId::from_bytes([31; 16]);
    let branch = EntityId::from_bytes([32; 16]);
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "new-session".to_owned(),
            response: Response::AgentAccepted(AgentAcceptedResponse {
                operation: OperationStatusResponse {
                    operation: Operation {
                        reference: scriberig_protocol::OperationReference {
                            id: "op_new_session_remote".to_owned(),
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
        .start_new_session_handle(&ApplicationContext::empty(), "first prompt", None)
        .expect("authoritative new-session handles");

    assert_eq!(handle.operation.to_string(), "op_new_session_remote");
    assert_eq!(handle.session, session);
    assert_eq!(handle.branch, Some(branch));
}

#[path = "service_remote_tests_part1.rs"]
mod part1;
