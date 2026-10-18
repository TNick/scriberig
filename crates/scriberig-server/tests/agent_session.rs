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
#[allow(clippy::too_many_lines)]
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
    let model_id = scriberig_app::ModelId::new("model-a").expect("model id");
    let library = scriberig_app::ModelLibrary::new(
        vec![scriberig_app::ModelDescriptor {
            id: model_id.clone(),
            display_name: "Model A".to_owned(),
            provider_profile: "provider-a".to_owned(),
            provider_model_id: "wire-model-a".to_owned(),
            capabilities: scriberig_app::ModelCapabilities::default(),
            thinking_levels: vec![scriberig_app::ThinkingLevelDescriptor {
                id: scriberig_app::ThinkingLevelId::new("deep-dive").expect("level id"),
                label: "Deep dive".to_owned(),
                description: None,
                order: 0,
            }],
            default_thinking_level: None,
        }],
        vec![scriberig_app::ModelProfile {
            id: "profile-a".to_owned(),
            display_name: "Primary".to_owned(),
            model: model_id,
            connection: "connection-a".to_owned(),
            credential: Some(scriberig_app::CredentialRef {
                id: "credential-ref-only".to_owned(),
            }),
            default_thinking_level: None,
            enabled: true,
        }],
    )
    .expect("model library");
    let service = ApplicationService::new(Fixture).with_model_library(library);
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
            model_profile: "profile-a".to_owned(),
        }),
    });

    let Response::SessionExecution(selection) = response.response else {
        panic!("unexpected response: {:?}", response.response);
    };
    assert_eq!(selection.revision, 1);
    assert_eq!(selection.model.as_deref(), Some("model-a"));
    assert_eq!(selection.thinking_level.as_deref(), Some("deep-dive"));
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

/// Verifies shared presentation preferences use trusted dispatcher identity.
#[test]
fn presentation_preferences_are_principal_scoped_and_revision_checked() {
    let user_a = Dispatcher::for_principal(
        ApplicationService::new(Fixture),
        scriberig_app::Principal::new("user-a"),
    );
    let read_request = || {
        Request::AgentPresentationPreferencesRead(
            scriberig_protocol::AgentPresentationPreferencesReadRequest {},
        )
    };
    let make_envelope = |request| RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "preferences-test".to_owned(),
        request,
    };
    let initial = user_a.dispatch(make_envelope(read_request()));
    let Response::AgentPresentationPreferences(initial) = initial.response else {
        panic!("unexpected preference response: {initial:?}");
    };
    assert_eq!(initial.revision, 0);

    let changed = user_a.dispatch(make_envelope(Request::AgentPresentationPreferencesSet(
        scriberig_protocol::AgentPresentationPreferencesSetRequest {
            expected_revision: initial.revision,
            preferences: scriberig_protocol::PublicAgentPresentationPreferences {
                tool_output: scriberig_protocol::PublicToolOutputVisibility::Full,
                ..scriberig_protocol::PublicAgentPresentationPreferences::default()
            },
        },
    )));
    let Response::AgentPresentationPreferences(changed) = changed.response else {
        panic!("unexpected preference update response");
    };
    assert_eq!(changed.revision, 1);
    assert_eq!(
        changed.preferences.tool_output,
        scriberig_protocol::PublicToolOutputVisibility::Full
    );

    let stale = user_a.dispatch(make_envelope(Request::AgentPresentationPreferencesSet(
        scriberig_protocol::AgentPresentationPreferencesSetRequest {
            expected_revision: initial.revision,
            preferences: scriberig_protocol::PublicAgentPresentationPreferences::default(),
        },
    )));
    assert!(matches!(
        stale.response,
        Response::Error(scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::Conflict,
            ..
        })
    ));
}

/// Verifies Rust client preference methods use the principal-aware transport.
#[test]
fn rust_client_reads_and_updates_shared_presentation_preferences() {
    let transport = scriberig_server::InProcessTransport::for_principal(
        ApplicationService::new(Fixture),
        scriberig_app::Principal::new("client-user"),
    );
    let client = scriberig_client::Client::new(transport);
    let initial = client
        .agent_presentation_preferences()
        .expect("read preferences");
    let updated = client
        .set_agent_presentation_preferences(
            initial.revision,
            scriberig_protocol::PublicAgentPresentationPreferences {
                prompt: scriberig_protocol::PublicPromptVisibility::Shown,
                ..scriberig_protocol::PublicAgentPresentationPreferences::default()
            },
        )
        .expect("update preferences");

    assert_eq!(updated.revision, initial.revision + 1);
    assert_eq!(
        updated.preferences.prompt,
        scriberig_protocol::PublicPromptVisibility::Shown
    );
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
