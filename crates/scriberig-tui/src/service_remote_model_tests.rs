//! Verifies remote model-control calls preserve shared request semantics.

use std::sync::{Arc, Mutex};

use super::super::{RemoteAdapter, TuiModelRequestService, TuiService};
use scriberig_app::{ApplicationContext, SessionModelSelectionRequest};
use scriberig_client::Client;
use scriberig_core::EntityId;
use scriberig_protocol::{
    AgentPresentationPreferencesResponse, ModelLibraryResponse, ProtocolTransport, ProtocolVersion,
    Request, RequestEnvelope, Response, ResponseEnvelope, SessionExecutionSelection,
};

/// Verifies the remote adapter reads one captured request by stable identity.
#[test]
fn reads_model_request_detail_through_protocol_client() {
    let observed = Arc::new(Mutex::new(None));
    let session = EntityId::from_bytes([11; 16]);
    let branch = EntityId::from_bytes([12; 16]);
    let invocation = EntityId::from_bytes([13; 16]);
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: Response::ModelRequestDetail(scriberig_protocol::ModelRequestDetailResponse {
            invocation: invocation.to_string(),
            model: "model-a".to_owned(),
            provider_id: Some("provider-a".to_owned()),
            provider_profile: None,
            thinking_level: Some("careful".to_owned()),
            profile: None,
            active_components: Vec::new(),
            context_fingerprint: None,
            request_fingerprint: None,
            catalog_revision: None,
            upstream_runtime_revision: None,
            provider_response_id: None,
            model_execution_configuration: None,
            messages: Vec::new(),
            tools: Vec::new(),
            applied_settings: None,
            applied_settings_fingerprint: None,
            truncated: false,
            content_complete: true,
            fragments: Vec::new(),
            offset: Some(0),
            total_chars: Some(0),
            next_offset: None,
        }),
        request: Arc::clone(&observed),
    }));

    let detail = adapter
        .model_request_detail(&ApplicationContext::empty(), session, branch, invocation)
        .expect("captured request detail");

    assert_eq!(detail.model, "model-a");
    assert_eq!(detail.provider_id.as_deref(), Some("provider-a"));
    assert_eq!(detail.thinking_level.as_deref(), Some("careful"));
    assert_eq!(
        observed.lock().expect("request").as_ref(),
        Some(&Request::ModelRequestDetail(
            scriberig_protocol::ModelRequestDetailRequest {
                session: scriberig_protocol::SessionReference {
                    id: session.to_string(),
                },
                branch: scriberig_protocol::BranchReference {
                    id: branch.to_string(),
                },
                invocation: invocation.to_string(),
                offset: Some(0),
                max_chars: Some(32_768),
            }
        ))
    );
}

/// Verifies TUI startup reads shared presentation defaults through the client.
#[test]
fn reads_shared_presentation_preferences_through_protocol_client() {
    let observed = Arc::new(Mutex::new(None));
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: Response::AgentPresentationPreferences(AgentPresentationPreferencesResponse {
            revision: 5,
            preferences: scriberig_protocol::PublicAgentPresentationPreferences {
                tool_output: scriberig_protocol::PublicToolOutputVisibility::Hidden,
                ..scriberig_protocol::PublicAgentPresentationPreferences::default()
            },
        }),
        request: Arc::clone(&observed),
    }));

    let preferences = adapter
        .agent_presentation_preferences(&ApplicationContext::empty())
        .expect("shared preferences");

    assert_eq!(preferences.revision, 5);
    assert_eq!(
        preferences.preferences.tool_output,
        scriberig_protocol::PublicToolOutputVisibility::Hidden
    );
    assert_eq!(
        observed.lock().expect("request").as_ref(),
        Some(&Request::AgentPresentationPreferencesRead(
            scriberig_protocol::AgentPresentationPreferencesReadRequest {}
        ))
    );
}

/// Returns a fixed protocol response and captures the request.
#[derive(Clone)]
struct FixtureTransport {
    /// Fixed response returned to the client.
    response: Response,
    /// Last typed request received by the transport.
    request: Arc<Mutex<Option<Request>>>,
}

impl ProtocolTransport for FixtureTransport {
    /// Captures the request and returns the configured response.
    ///
    /// - `self`: Deterministic protocol fixture.
    /// - `request`: Correlated request sent by the client.
    ///
    /// The fixed response with the request's correlation identity.
    /// # Returns
    /// The operation result.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        if let Ok(mut observed) = self.request.lock() {
            *observed = Some(request.request);
        }
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: self.response.clone(),
        }
    }
}

/// Verifies the model library request uses the public client contract.
#[test]
fn reads_model_library_through_protocol_client() {
    let observed = Arc::new(Mutex::new(None));
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: Response::ModelLibrary(ModelLibraryResponse::default()),
        request: Arc::clone(&observed),
    }));

    let library = adapter
        .model_library(&ApplicationContext::empty())
        .expect("model library");

    assert_eq!(library.models, Vec::new());
    assert_eq!(
        observed.lock().expect("request").as_ref(),
        Some(&Request::ModelLibrary(
            scriberig_protocol::ModelLibraryRequest::default()
        ))
    );
}

/// Verifies model selection preserves session and optimistic revision values.
#[test]
fn sets_session_model_with_expected_revision() {
    let observed = Arc::new(Mutex::new(None));
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: Response::SessionExecution(SessionExecutionSelection {
            revision: 5,
            provider_id: Some("provider-a".to_owned()),
            model_id: Some("model-a".to_owned()),
            thinking_level: Some("careful".to_owned()),
            history: Vec::new(),
        }),
        request: Arc::clone(&observed),
    }));
    let session = EntityId::from_bytes([9; 16]);

    let updated = adapter
        .set_session_model(
            &ApplicationContext::empty(),
            &SessionModelSelectionRequest {
                session,
                expected_revision: 4,
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
            },
        )
        .expect("updated selection");

    assert_eq!(updated.revision, 5);
    assert_eq!(updated.provider_id.as_deref(), Some("provider-a"));
    assert_eq!(updated.model_id.as_deref(), Some("model-a"));
    assert_eq!(
        observed.lock().expect("request").as_ref(),
        Some(&Request::SessionModelSelection(
            scriberig_protocol::SessionModelSelectionRequest {
                session: scriberig_protocol::SessionReference {
                    id: session.to_string(),
                },
                expected_revision: 4,
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
            }
        ))
    );
}
