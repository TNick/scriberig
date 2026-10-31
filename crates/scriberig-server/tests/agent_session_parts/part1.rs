//! Covers additional focused cases for the parent test module.

use super::*;

#[path = "part1/remote_plan.rs"]
mod remote_plan;

/// Verifies presentation preferences are principal scoped and revision checked.
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
    assert_eq!(value.cad_providers.len(), 1);
    assert_eq!(
        value.cad_providers[0].availability,
        scriberig_protocol::CadProviderAvailability::Unavailable
    );
    assert_eq!(
        value.cad_providers[0].format,
        scriberig_protocol::CadProviderFormat::Dwg
    );
    assert!(!value.cad_providers[0].can_write);
}

/// Verifies embedded and transport-backed agent results remain deterministic.
#[test]
fn embedded_and_client_agent_results_are_conformant() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/conformance/fixtures/agent-operation.json"
    ))
    .expect("shared browser/server agent fixture");
    let session = scriberig_core::EntityId::from_bytes([2; 16]);
    let session_id = session.to_string();
    let prompt = fixture["request"]["prompt"]
        .as_str()
        .expect("fixture prompt");
    assert_eq!(
        fixture["request"]["sessionId"].as_str(),
        Some(session_id.as_str())
    );
    let direct = ApplicationService::new(Fixture);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_start").expect("capability"),
            CapabilityId::new("agent_continue").expect("capability"),
        ]),
        ..ApplicationContext::empty()
    };
    let direct_operation = direct
        .start_agent(&context, session, prompt)
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
        .start_agent(session_id, prompt)
        .expect("transport operation");
    let remote_result = response.operation.agent_result.expect("remote result");

    assert_eq!(remote_result.final_output, direct_result.final_output);
    assert_eq!(
        direct_result.final_output,
        fixture["expected"]["finalOutput"]
            .as_str()
            .expect("fixture final output")
    );
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

/// Routes plan creation, lifecycle, assignment, and reads through app operations.
#[test]
#[allow(clippy::too_many_lines)] // This scenario checks all plan transport routes together.
fn protocol_plan_handlers_delegate_to_application_operations() {
    let dispatcher = Dispatcher::new(ApplicationService::new(Fixture));
    let session = "ent_51515151515151515151515151515151".to_owned();
    let started = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "seed-plan-session".to_owned(),
        request: Request::AgentStart(AgentStartRequest {
            session: SessionReference {
                id: session.clone(),
            },
            prompt: "plan root".to_owned(),
            selection: None,
        }),
    });
    let Response::AgentAccepted(started) = started.response else {
        panic!("unexpected start response");
    };
    let root = started.branch.expect("root branch");
    let created = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "create-plan".to_owned(),
        request: Request::PlanCreate(SessionPlanCreateRequest {
            session: SessionReference {
                id: session.clone(),
            },
            anchor: SessionContinuationAnchor::BranchHead {
                branch: root.id.clone(),
            },
            goal: "finish task".to_owned(),
            nodes: vec![SessionPlanNodeDraft {
                key: "work".to_owned(),
                parent_index: None,
                title: "Work".to_owned(),
                purpose: "perform task".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            }],
            dependencies: Vec::new(),
        }),
    });
    let Response::Plan(created) = created.response else {
        panic!("unexpected plan creation response");
    };
    assert!(created.exact_revision.is_some());
    assert!(created.nodes[0].exact_revision.is_some());
    assert_eq!(
        created.status,
        scriberig_protocol::SessionPlanStatus::Planned
    );
    let started_plan = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "start-plan".to_owned(),
        request: Request::PlanLifecycle(SessionPlanLifecycleRequest {
            session: SessionReference {
                id: session.clone(),
            },
            plan: created.id.clone(),
            expected_revision: created.revision,
            action: SessionPlanAction::Start,
        }),
    });
    let Response::Plan(started_plan) = started_plan.response else {
        panic!("unexpected plan lifecycle response");
    };
    assert_eq!(
        started_plan.status,
        scriberig_protocol::SessionPlanStatus::Running
    );
    assert!(started_plan.coordinator_claim.is_some());
    let appended = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "append-plan-step".to_owned(),
        request: Request::PlanMutation(SessionPlanMutationRequest {
            session: SessionReference {
                id: session.clone(),
            },
            plan: started_plan.id.clone(),
            expected_revision: started_plan.revision,
            mutation: SessionPlanMutation::AddNextStep {
                node: SessionPlanNodeDraft {
                    key: "follow-up".to_owned(),
                    parent_index: None,
                    title: "Follow up".to_owned(),
                    purpose: "complete the remaining work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                },
            },
        }),
    });
    let Response::Plan(appended) = appended.response else {
        panic!("unexpected plan mutation response");
    };
    assert_eq!(appended.revision, started_plan.revision + 1);
    assert_eq!(appended.nodes.len(), 2);
    let paused = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "pause-plan".to_owned(),
        request: Request::PlanLifecycle(SessionPlanLifecycleRequest {
            session: SessionReference {
                id: session.clone(),
            },
            plan: appended.id.clone(),
            expected_revision: appended.revision,
            action: SessionPlanAction::Pause,
        }),
    });
    let Response::Plan(paused) = paused.response else {
        panic!("unexpected pause response");
    };
    let assigned = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "assign-plan-node".to_owned(),
        request: Request::PlanAssignment(SessionPlanAssignmentRequest {
            session: SessionReference {
                id: session.clone(),
            },
            plan: paused.id.clone(),
            node: paused.nodes[0].id.clone(),
            expected_revision: paused.revision,
            assignment: None,
        }),
    });
    let Response::Plan(assigned) = assigned.response else {
        panic!("unexpected assignment response");
    };
    assert_eq!(assigned.revision, paused.revision + 1);
    let read = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "read-plan".to_owned(),
        request: Request::SessionRead(SessionReadRequest {
            session: SessionReference { id: session },
            branch: None,
        }),
    });
    let Response::Session(read) = read.response else {
        panic!("unexpected session read response");
    };
    let mut assigned_without_history = assigned;
    assigned_without_history.history.clear();
    assert_eq!(read.plans[0], assigned_without_history);
}
