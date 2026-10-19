//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies local and remote session adapters share semantics.
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
                plans: Vec::new(),
                execution: scriberig_protocol::SessionExecutionSelection::default(),
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
                        activity: Vec::new(),
                        result: None,
                    },
                    SessionBranch {
                        id: child.to_string(),
                        parent: Some(root.to_string()),
                        children: Vec::new(),
                        title: "focused".to_owned(),
                        status: SessionBranchStatus::Completed,
                        turns: Vec::new(),
                        activity: Vec::new(),
                        result: None,
                    },
                ],
                plans: Vec::new(),
                execution: scriberig_protocol::SessionExecutionSelection::default(),
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
    /// - `self`: Cursor-aware fixture transport.
    /// - `request`: Protocol subscription request.
    ///
    /// One operation-completed event at sequence five.
    /// # Returns
    /// The operation result.
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
                activity_kind: None,
                activity_state: None,
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
