//! Reads durable request and tool-result detail by stable activity identity.

use scriberig_app::{
    AgentConsoleBackend, ApplicationContext, ApplicationService, CapabilityContext, CapabilityId,
    SessionBackend,
};
use scriberig_protocol::{ProtocolVersion, Request, RequestEnvelope, Response, SessionReference};
use scriberig_server::Dispatcher;

use super::fixture::Fixture;

/// Reads captured requests and safe result detail by stable activity identity.
#[test]
fn durable_detail_reads_do_not_reexecute_tools_or_models() {
    let service = ApplicationService::new(Fixture);
    let handle = service
        .start_local_agent(&scriberig_app::AgentCommandRequest {
            session: scriberig_core::EntityId::from_bytes([31; 16]),
            branch: None,
            anchor: None,
            prompt: "inspect safely".to_owned(),
            selection: None,
        })
        .expect("start local agent");
    service
        .complete_local_agent_with_evidence(
            &handle,
            "done",
            Vec::new(),
            vec![scriberig_app::ToolExecutionEvidence {
                ordinal: 1,
                name: "inspect".to_owned(),
                input: br"{}".to_vec(),
                presentation_input: None,
                presentation_label: None,
                category: Some(scriberig_session::ToolActivityKind::SemanticRead),
                output: serde_json::to_vec(&serde_json::json!({
                    "summary": "s".repeat(6_000),
                    "private_payload": "FAKE_PRIVATE_RESULT_8391",
                }))
                .expect("oversized structured result"),
                failed: false,
                child_branch: None,
            }],
            vec![scriberig_session::ModelRequestProjection {
                ordinal: 0,
                model: "model-a".to_owned(),
                provider_id: Some("provider-a".to_owned()),
                provider_profile: None,
                thinking_level: Some("careful".to_owned()),
                profile: Some(scriberig_session::ModelContextRevisionRef {
                    identity: "profile.default".to_owned(),
                    revision: "rev-3".to_owned(),
                }),
                active_components: vec![scriberig_session::ModelContextRevisionRef {
                    identity: "component.core".to_owned(),
                    revision: "rev-8".to_owned(),
                }],
                context_fingerprint: None,
                request_fingerprint: None,
                model_execution_configuration: None,
                messages: vec![scriberig_session::ModelRequestMessage {
                    role: "user".to_owned(),
                    content: "inspect safely".to_owned(),
                }],
                full_messages: None,
                tools: Vec::new(),
                full_tools: None,
                content_complete: false,
                applied_settings: None,
                applied_settings_fingerprint: None,
                truncated: false,
            }],
        )
        .expect("complete local agent");

    let read_context = ApplicationContext {
        capabilities: CapabilityContext::new(vec![
            CapabilityId::new("session_branch_tree").expect("session read capability"),
        ]),
        ..ApplicationContext::empty()
    };
    let branch = service
        .read_session(
            &read_context,
            &scriberig_app::SessionRequest {
                session: handle.session,
            },
        )
        .expect("read durable session")
        .branches[0]
        .clone();
    assert_details_are_read_only(&service, &handle, &read_context, &branch);
}

/// Verifies public detail reads return bounded cached data without execution.
///
/// - `service`: Application service holding completed session evidence.
/// - `handle`: Accepted local-agent session identities.
/// - `read_context`: Authorized session read context.
/// - `branch`: Durable branch containing the captured activity.
fn assert_details_are_read_only(
    service: &ApplicationService<Fixture>,
    handle: &scriberig_app::LocalAgentHandle,
    read_context: &ApplicationContext,
    branch: &scriberig_session::Branch,
) {
    let dispatcher = Dispatcher::new(service);
    let shared_detail = service
        .model_request_detail(
            read_context,
            handle.session,
            handle.root_branch,
            branch.model_invocations[0].id,
        )
        .expect("shared application request detail");
    assert_eq!(shared_detail.model, "model-a");
    assert_eq!(shared_detail.provider_id.as_deref(), Some("provider-a"));
    assert_eq!(
        shared_detail
            .profile
            .as_ref()
            .map(|reference| (reference.identity.as_str(), reference.revision.as_str())),
        Some(("profile.default", "rev-3"))
    );
    assert_eq!(shared_detail.active_components.len(), 1);
    assert_eq!(shared_detail.messages[0].content, "inspect safely");
    let request = Request::ModelRequestDetail(scriberig_protocol::ModelRequestDetailRequest {
        session: SessionReference {
            id: handle.session.to_string(),
        },
        branch: scriberig_protocol::BranchReference {
            id: handle.root_branch.to_string(),
        },
        invocation: branch.model_invocations[0].id.to_string(),
        offset: None,
        max_chars: None,
    });
    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "request-detail".to_owned(),
        request,
    });
    let Response::ModelRequestDetail(detail) = response.response else {
        panic!("unexpected response: {:?}", response.response);
    };
    assert_eq!(detail.model, "model-a");
    assert_eq!(detail.provider_id.as_deref(), Some("provider-a"));
    assert_eq!(detail.thinking_level.as_deref(), Some("careful"));
    assert_eq!(
        detail
            .profile
            .as_ref()
            .map(|reference| (reference.identity.as_str(), reference.revision.as_str())),
        Some(("profile.default", "rev-3"))
    );
    assert_eq!(detail.active_components.len(), 1);
    assert_eq!(detail.messages[0].content, "inspect safely");

    let result_id = branch.tool_results[0].id.to_string();
    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "tool-result-detail".to_owned(),
        request: Request::ToolResultDetail(scriberig_protocol::ToolResultDetailRequest {
            session: SessionReference {
                id: handle.session.to_string(),
            },
            branch: scriberig_protocol::BranchReference {
                id: handle.root_branch.to_string(),
            },
            result: result_id.clone(),
        }),
    });
    let Response::ToolResultDetail(detail) = response.response else {
        panic!("unexpected response: {:?}", response.response);
    };
    assert_eq!(detail.result, result_id);
    assert_eq!(
        detail.detail["summary"].as_str().unwrap().chars().count(),
        4096
    );
    assert!(detail.truncated);
    assert!(detail.detail.get("private_payload").is_none());

    let response = dispatcher.dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "session-activity".to_owned(),
        request: Request::SessionRead(scriberig_protocol::SessionReadRequest {
            session: SessionReference {
                id: handle.session.to_string(),
            },
            branch: None,
        }),
    });
    let Response::Session(session) = response.response else {
        panic!("unexpected response: {:?}", response.response);
    };
    let activity = session.branches[0]
        .activity
        .iter()
        .find(|item| item.kind == scriberig_protocol::SessionActivityKind::ToolResult)
        .expect("bounded tool result activity");
    assert_eq!(activity.durable_ref.as_deref(), Some(result_id.as_str()));
    assert_eq!(activity.summary.chars().count(), 513);
    assert!(activity.summary.ends_with('…'));
    assert!(activity.tool_result_preview.is_some());
}
