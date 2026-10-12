//! Verifies local and remote TUI adapters expose equivalent semantics.

use std::sync::{Arc, Mutex};

use super::super::{ApplicationAdapter, RemoteAdapter};
use crate::service::TuiService;
use scriberig_app::{
    AgentCommandRequest, ApplicationContext, ApplicationError, InspectRequest, InspectResult,
    QueryService, SessionBackend, SessionRequest,
};
use scriberig_client::Client;
use scriberig_core::EntityId;
use scriberig_protocol::{
    Event, EventKind, OperationReference, ProtocolTransport, ProtocolVersion, Request,
    RequestEnvelope, Response, ResponseEnvelope, SessionBranch, SessionBranchStatus,
    SessionReadResponse, SessionReference, SessionTurn,
};
use scriberig_session::{AgentOutput, Branch, HumanInput, Session, Turn};

/// Supplies one deterministic local session to the application adapter.
#[derive(Clone)]
struct LocalSessionFixture {
    /// Authoritative session returned by the local boundary.
    session: Session,
}

impl QueryService for LocalSessionFixture {
    /// Returns the deterministic inspection represented by both adapters.
    ///
    /// # Parameters
    /// - `_context`: Application request context.
    /// - `request`: Inspection request to resolve.
    ///
    /// # Returns
    /// The inspection preserves the requested semantic handles.
    ///
    /// # Errors
    /// This fixture does not return an application error.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Ok(InspectResult {
            entity: request.entity,
            revision: Some(scriberig_core::RevisionId::new([8; 32])),
            snapshot: match request.snapshot {
                scriberig_app::SnapshotResolution::At(snapshot) => Some(snapshot),
                _ => None,
            },
            object_type: Some(
                scriberig_core::ObjectType::new("fixture.entity").expect("object type"),
            ),
            label: Some("fixture inspection".to_owned()),
            description: None,
            capabilities: Vec::new(),
            trust: None,
        })
    }
}

impl SessionBackend for LocalSessionFixture {
    /// Returns the deterministic session snapshot.
    ///
    /// # Parameters
    /// - `_context`: Application request context.
    /// - `request`: Session identity requested by the adapter.
    ///
    /// # Returns
    /// The fixture session when the identity matches.
    ///
    /// # Errors
    /// Returns not-found for another session identity.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        request: &SessionRequest,
    ) -> Result<Session, ApplicationError> {
        if request.session == self.session.id {
            Ok(self.session.clone())
        } else {
            Err(ApplicationError::NotFound {
                subject: request.session.to_string(),
            })
        }
    }

    /// Returns one deterministic operation for adapter handle tests.
    ///
    /// # Parameters
    /// - `_context`: Application request context.
    /// - `_request`: Accepted agent request.
    ///
    /// # Returns
    /// A stable fixture operation identity.
    ///
    /// # Errors
    /// This fixture never rejects submission.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _request: &AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        Ok(scriberig_core::OperationId::new("op_local_fixture").expect("operation"))
    }
}

/// Supplies the fixed protocol response used by this adapter test.
#[derive(Clone)]
struct FixtureTransport {
    /// Response returned to every protocol request.
    response: ResponseEnvelope,
}

impl ProtocolTransport for FixtureTransport {
    /// Returns the deterministic response envelope.
    ///
    /// # Parameters
    /// - `self`: Fixture transport.
    /// - `_request`: Protocol request received by the fixture.
    ///
    /// # Returns
    /// The configured protocol response.
    fn send(&self, _request: RequestEnvelope) -> ResponseEnvelope {
        self.response.clone()
    }
}

/// Builds the semantic session represented by the protocol fixture.
///
/// # Returns
/// A deterministic session with one active branch and one completed turn.
fn session_fixture() -> Session {
    let session = EntityId::from_bytes([5; 16]);
    let branch = EntityId::from_bytes([6; 16]);
    let turn = EntityId::from_bytes([7; 16]);
    let mut root = Branch::new(branch, "main").expect("branch");
    root.push_turn(Turn {
        id: turn,
        ordinal: 3,
        human: Some(HumanInput {
            text: "inspect".to_owned(),
        }),
        agent: Some(AgentOutput {
            text: "fixture output".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    Session {
        id: session,
        branches: vec![root],
        goals: Vec::new(),
        tasks: Vec::new(),
    }
}

/// Verifies local and remote inspection preserve the same semantic result.
#[test]
fn local_and_remote_inspection_share_semantics() {
    let entity = EntityId::from_bytes([30; 16]);
    let snapshot = scriberig_core::SnapshotId::new([31; 32]);
    let local = ApplicationAdapter::new(
        LocalSessionFixture {
            session: session_fixture(),
        },
        Vec::new(),
    )
    .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
    .expect("local inspection");
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "inspect-equivalence".to_owned(),
            response: Response::Inspect(scriberig_protocol::InspectEntityResponse {
                entity: scriberig_protocol::EntityReference {
                    id: entity.to_string(),
                },
                revision: scriberig_protocol::RevisionReference {
                    entity: scriberig_protocol::EntityReference {
                        id: entity.to_string(),
                    },
                    id: scriberig_core::RevisionId::new([8; 32]).to_string(),
                },
                snapshot: scriberig_protocol::SnapshotReference {
                    id: snapshot.to_string(),
                },
                object_type: Some("fixture.entity".to_owned()),
                summary: Some("fixture inspection".to_owned()),
            }),
        },
    };
    let remote = RemoteAdapter::new(Client::new(transport))
        .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
        .expect("remote inspection");

    assert_eq!(local.entity, remote.entity);
    assert_eq!(local.revision, remote.revision);
    assert_eq!(local.snapshot, remote.snapshot);
    assert_eq!(local.object_type, remote.object_type);
    assert_eq!(local.label, remote.label);
}

/// Verifies local and remote adapters reconstruct the same session semantics.
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
                    result: None,
                }],
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
}

/// Verifies remote nested branch references become the shared session graph.
#[test]
fn remote_nested_branch_tree_preserves_parent_and_child_handles() {
    let session = EntityId::from_bytes([20; 16]);
    let root = EntityId::from_bytes([21; 16]);
    let child = EntityId::from_bytes([22; 16]);
    let transport = FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "nested-session".to_owned(),
            response: Response::Session(SessionReadResponse {
                session: SessionReference {
                    id: session.to_string(),
                },
                branches: vec![
                    SessionBranch {
                        id: root.to_string(),
                        parent: None,
                        children: vec![child.to_string()],
                        title: "root".to_owned(),
                        status: SessionBranchStatus::Active,
                        turns: Vec::new(),
                        result: None,
                    },
                    SessionBranch {
                        id: child.to_string(),
                        parent: Some(root.to_string()),
                        children: Vec::new(),
                        title: "focused".to_owned(),
                        status: SessionBranchStatus::Completed,
                        turns: Vec::new(),
                        result: None,
                    },
                ],
            }),
        },
    };
    let session = RemoteAdapter::new(Client::new(transport))
        .read_session(&ApplicationContext::empty(), session)
        .expect("nested session");

    assert_eq!(session.branches.len(), 2);
    assert_eq!(session.branches[0].children, vec![child]);
    assert_eq!(session.branches[1].parent, Some(root));
    assert_eq!(
        session.branches[1].status,
        scriberig_session::BranchStatus::Completed
    );
}

/// Records subscription cursors while returning one durable operation event.
#[derive(Clone)]
struct CursorTransport {
    /// Requests received by the transport.
    requests: Arc<Mutex<Vec<RequestEnvelope>>>,
}

impl ProtocolTransport for CursorTransport {
    /// Records the request and returns one stable completion event.
    ///
    /// # Parameters
    /// - `self`: Cursor-aware fixture transport.
    /// - `request`: Protocol subscription request.
    ///
    /// # Returns
    /// One operation-completed event at sequence five.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        self.requests.lock().expect("request log").push(request);
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "cursor".to_owned(),
            response: Response::Events(vec![Event {
                sequence: 5,
                kind: EventKind::OperationCompleted,
                entity: None,
                revision: None,
                snapshot: None,
                operation: Some(OperationReference {
                    id: "op-1".to_owned(),
                }),
                metadata: Vec::new(),
            }]),
        }
    }
}

/// Verifies remote reconnect polling resumes from the last durable cursor.
#[test]
fn remote_event_polling_resumes_from_last_cursor() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let adapter = RemoteAdapter::new(Client::new(CursorTransport {
        requests: Arc::clone(&requests),
    }));
    adapter
        .application_events(&ApplicationContext::empty())
        .expect("first event poll");
    adapter
        .application_events(&ApplicationContext::empty())
        .expect("resumed event poll");

    let requests = requests.lock().expect("request log");
    let after = requests
        .iter()
        .map(|request| match &request.request {
            Request::Subscribe(value) => value.after,
            other => panic!("unexpected request: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(after, vec![None, Some(5)]);
}
