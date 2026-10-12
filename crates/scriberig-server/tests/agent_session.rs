//! Verifies protocol agent start creates inspectable session branch state.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, CapabilityContext, CapabilityId,
};
use scriberig_protocol::{
    AgentStartRequest, OperationReference, OperationStatusRequest, ProtocolVersion, Request,
    RequestEnvelope, Response, RuntimeCapabilitiesRequest, SessionReadRequest, SessionReference,
    SubscribeRequest,
};
use scriberig_server::Dispatcher;

/// Leaves agent/session ownership to the application service fallback.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal operations because this test targets agent fallback.
    ///
    /// # Parameters
    /// - `_context`: Application context, unused by the fixture.
    /// - `_request`: Universal request, intentionally unsupported.
    ///
    /// # Returns
    /// Never returns a universal result.
    ///
    /// # Errors
    /// Always returns unsupported-operation.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "fixture universal request".to_owned(),
        })
    }
}

/// Verifies start returns operation, session, and root-branch handles.
#[test]
fn agent_start_returns_inspectable_branch() {
    let dispatcher = Dispatcher::new(ApplicationService::new(Fixture));
    let session = "ent_01010101010101010101010101010101".to_owned();
    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "start".to_owned(),
        request: Request::AgentStart(AgentStartRequest {
            session: SessionReference {
                id: session.clone(),
            },
            prompt: "inspect".to_owned(),
        }),
    });
    let (operation_id, branch) = match response.response {
        Response::AgentAccepted(value) => {
            assert_eq!(value.session.id, session);
            assert_eq!(
                value.operation.operation.status,
                scriberig_protocol::OperationStatus::Completed
            );
            let result = value
                .operation
                .operation
                .agent_result
                .expect("terminal agent result");
            assert_eq!(result.session.id, session);
            assert_eq!(result.final_output, "inspect");
            (
                value.operation.operation.reference.id,
                value.branch.expect("root branch handle"),
            )
        }
        other => panic!("unexpected response: {other:?}"),
    };
    assert_ne!(branch.id, "");

    // Simulate dropping the event connection; durable reads remain authoritative.
    let _ = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "dropped-events".to_owned(),
        request: Request::Subscribe(SubscribeRequest {
            event_kinds: Vec::new(),
            after: None,
        }),
    });

    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "read".to_owned(),
        request: Request::SessionRead(SessionReadRequest {
            session: SessionReference {
                id: session.clone(),
            },
            branch: None,
        }),
    });
    match response.response {
        Response::Session(value) => {
            assert_eq!(value.branches.len(), 1);
            assert_eq!(value.branches[0].id, branch.id);
            assert_eq!(value.branches[0].turns.len(), 1);
            assert_eq!(
                value.branches[0].turns[0].agent_output.as_deref(),
                Some("inspect")
            );
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "branch-read".to_owned(),
        request: Request::SessionRead(SessionReadRequest {
            session: SessionReference {
                id: session.clone(),
            },
            branch: Some(scriberig_protocol::BranchReference {
                id: branch.id.clone(),
            }),
        }),
    });
    match response.response {
        Response::Session(value) => {
            assert_eq!(value.branches.len(), 1);
            assert_eq!(value.branches[0].id, branch.id);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "status".to_owned(),
        request: Request::OperationStatus(OperationStatusRequest {
            operation: OperationReference { id: operation_id },
        }),
    });
    match response.response {
        Response::OperationStatus(value) => {
            assert_eq!(
                value.operation.status,
                scriberig_protocol::OperationStatus::Completed
            );
            assert!(value.operation.agent_result.is_some());
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

/// Keeps the protocol request import in the test's public-surface coverage.
#[test]
fn capabilities_request_remains_constructible() {
    let _ = Request::Capabilities(RuntimeCapabilitiesRequest::default());
}

/// Verifies agent dispatch capabilities are advertised by the same server.
#[test]
fn agent_capabilities_are_discoverable_before_submission() {
    let dispatcher = Dispatcher::new(ApplicationService::new(Fixture));
    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "capability-discovery".to_owned(),
        request: Request::Capabilities(RuntimeCapabilitiesRequest::default()),
    });
    let Response::Capabilities(value) = response.response else {
        panic!("unexpected capability response");
    };

    for expected in ["agent_start", "agent_continue"] {
        assert!(
            value
                .capabilities
                .iter()
                .any(|capability| capability.id == expected && capability.available)
        );
    }
}

/// Verifies embedded and transport-backed agent results remain deterministic.
#[test]
fn embedded_and_client_agent_results_are_conformant() {
    let session = scriberig_core::EntityId::from_bytes([2; 16]);
    let direct = ApplicationService::new(Fixture);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_start").expect("capability"),
            CapabilityId::new("agent_continue").expect("capability"),
        ]),
        ..ApplicationContext::empty()
    };
    let direct_operation = direct
        .start_agent(&context, session, "same prompt")
        .expect("direct operation");
    let direct_result = direct
        .operations()
        .get(&direct_operation)
        .expect("direct record")
        .agent_result
        .expect("direct result");

    let transport = scriberig_server::InProcessTransport::new(ApplicationService::new(Fixture));
    let remote = scriberig_client::Client::new(transport);
    let response = remote
        .start_agent(session.to_string(), "same prompt")
        .expect("transport operation");
    let remote_result = response.operation.agent_result.expect("remote result");

    assert_eq!(remote_result.final_output, direct_result.final_output);
    assert_eq!(remote_result.session.id, direct_result.session.to_string());
    assert_eq!(
        remote_result.root_branch.id,
        direct_result.root_branch.to_string()
    );
    assert_eq!(
        remote_result.branch_result,
        direct_result.branch_result.to_string()
    );
}
