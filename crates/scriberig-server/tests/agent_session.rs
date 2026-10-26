//! Verifies protocol agent start creates inspectable session branch state.

use rpi_ai::{Api, Model, ThinkingLevel, ThinkingLevelMap};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, CapabilityContext, CapabilityId,
};
use scriberig_protocol::{
    AgentContinueAtRequest, AgentNewSessionRequest, AgentStartRequest, OperationReference,
    OperationStatusRequest, ProtocolVersion, Request, RequestEnvelope, Response,
    RuntimeCapabilitiesRequest, SessionContinuationAnchor, SessionPlanAction,
    SessionPlanAssignmentRequest, SessionPlanCreateRequest, SessionPlanLifecycleRequest,
    SessionPlanMutation, SessionPlanMutationRequest, SessionPlanNodeDraft, SessionReadRequest,
    SessionReference, SubscribeRequest,
};
use scriberig_server::Dispatcher;

/// Leaves agent/session ownership to the application service fallback.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal operations because this test targets agent fallback.
    ///
    /// - `_context`: Application context, unused by the fixture.
    /// - `_request`: Universal request, intentionally unsupported.
    ///
    /// Never returns a universal result.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
#[allow(clippy::too_many_lines)]
fn agent_start_returns_inspectable_branch() {
    let dispatcher = Dispatcher::new(ApplicationService::new(Fixture));
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/conformance/fixtures/agent-operation.json"
    ))
    .expect("shared browser/server agent fixture");
    let session = fixture["request"]["sessionId"]
        .as_str()
        .expect("fixture session")
        .to_owned();
    let prompt = fixture["request"]["prompt"]
        .as_str()
        .expect("fixture prompt");
    let expected_output = fixture["expected"]["finalOutput"]
        .as_str()
        .expect("fixture final output");
    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "start".to_owned(),
        request: Request::AgentStart(AgentStartRequest {
            session: SessionReference {
                id: session.clone(),
            },
            prompt: prompt.to_owned(),
            selection: None,
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
            assert_eq!(result.final_output, expected_output);
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
                Some(expected_output)
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

/// Creates a session remotely and continues from an exact durable anchor.
#[test]
fn protocol_new_session_and_exact_continuation_use_shared_handles() {
    let dispatcher = Dispatcher::new(ApplicationService::new(Fixture));
    let started = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "new-session".to_owned(),
        request: Request::AgentNewSession(AgentNewSessionRequest {
            prompt: "first prompt".to_owned(),
            selection: None,
        }),
    });
    let Response::AgentAccepted(started) = started.response else {
        panic!("unexpected new-session response");
    };
    let root = started.branch.expect("authoritative root handle");
    let continued = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "exact-follow-up".to_owned(),
        request: Request::AgentContinueAt(AgentContinueAtRequest {
            session: started.session.clone(),
            anchor: SessionContinuationAnchor::BranchHead {
                branch: root.id.clone(),
            },
            prompt: "follow up".to_owned(),
            selection: None,
        }),
    });
    let Response::AgentAccepted(continued) = continued.response else {
        panic!("unexpected continuation response: {:?}", continued.response);
    };
    assert_eq!(continued.session, started.session);
    let branch = continued.branch.expect("continuation branch");
    assert_ne!(branch.id, root.id);
    let read = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "read-after-reconnect".to_owned(),
        request: Request::SessionRead(SessionReadRequest {
            session: started.session,
            branch: None,
        }),
    });
    let Response::Session(session) = read.response else {
        panic!("unexpected session response");
    };
    assert!(session.branches.iter().any(|item| item.id == branch.id));
}

/// Keeps the protocol request import in the test's public-surface coverage.
#[test]
fn capabilities_request_remains_constructible() {
    let _ = Request::Capabilities(RuntimeCapabilitiesRequest::default());
}

/// Verifies the public model-library request reaches the shared app operation.
#[test]
fn model_library_request_is_dispatched() {
    let dispatcher = Dispatcher::new(ApplicationService::new(Fixture));
    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "model-library".to_owned(),
        request: Request::ModelLibrary(scriberig_protocol::ModelLibraryRequest::default()),
    });

    assert_eq!(
        response.response,
        Response::ModelLibrary(scriberig_protocol::ModelLibraryResponse::default())
    );
}

/// Verifies a model selection persists through the public protocol session read.
#[test]
fn model_selection_mutation_round_trips_through_server() {
    let mut upstream_model = Model::new(
        "wire-model-a",
        "Model A",
        Api::AnthropicMessages,
        "provider-a",
        "https://example.test",
    );
    upstream_model.thinking_level_map = Some(ThinkingLevelMap::from([(
        ThinkingLevel::Xhigh,
        Some("xhigh".to_owned()),
    )]));
    let service = ApplicationService::new(Fixture).with_upstream_models(vec![upstream_model]);
    let dispatcher = Dispatcher::new(&service);
    let session = "ent_01010101010101010101010101010101".to_owned();
    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "select-model".to_owned(),
        request: Request::SessionModelSelection(scriberig_protocol::SessionModelSelectionRequest {
            session: SessionReference {
                id: session.clone(),
            },
            expected_revision: 0,
            provider_id: "provider-a".to_owned(),
            model_id: "wire-model-a".to_owned(),
        }),
    });

    let Response::SessionExecution(selection) = response.response else {
        panic!("unexpected response: {:?}", response.response);
    };
    assert_eq!(selection.revision, 1);
    assert_eq!(selection.provider_id.as_deref(), Some("provider-a"));
    assert_eq!(selection.model_id.as_deref(), Some("wire-model-a"));
    assert_eq!(selection.thinking_level.as_deref(), Some("xhigh"));
    let encoded_selection = serde_json::to_string(&selection).expect("serialize selection");
    assert!(!encoded_selection.contains("credential"));

    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "read-selection".to_owned(),
        request: Request::SessionRead(SessionReadRequest {
            session: SessionReference { id: session },
            branch: None,
        }),
    });
    let Response::Session(session) = response.response else {
        panic!("unexpected response: {:?}", response.response);
    };
    assert_eq!(session.execution, selection);
}

#[path = "agent_session_parts/attempts.rs"]
mod attempts;
#[path = "agent_session_parts/details.rs"]
mod details;
#[path = "agent_session_parts/part1.rs"]
mod part1;
