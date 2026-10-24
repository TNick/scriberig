//! Shared fixture backend and protocol detail helpers for durable detail reads.
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService,
};
use scriberig_protocol::{ProtocolVersion, Request, RequestEnvelope, Response, SessionReference};
use scriberig_server::Dispatcher;

/// Leaves local session state to the application service fallback.
pub(super) struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal operations because this test targets durable details.
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

/// Builds one durable request projection with the selected applied-settings state.
///
/// - `model`: Model identity recorded on the projection.
/// - `applied_settings`: Opaque upstream report to persist, when any.
/// - `catalog_configuration`: Catalog-derived execution metadata, when any.
///
/// # Returns
/// A bounded request projection for one model invocation.
pub(super) fn projection_with_settings(
    model: &str,
    applied_settings: Option<serde_json::Value>,
    catalog_configuration: Option<serde_json::Value>,
) -> scriberig_session::ModelRequestProjection {
    scriberig_session::ModelRequestProjection {
        ordinal: 0,
        model: model.to_owned(),
        provider_id: Some("provider-a".to_owned()),
        provider_profile: Some("host-profile".to_owned()),
        thinking_level: Some("off".to_owned()),
        profile: None,
        active_components: Vec::new(),
        context_fingerprint: None,
        request_fingerprint: Some("request-fingerprint".to_owned()),
        model_execution_configuration: catalog_configuration,
        messages: vec![scriberig_session::ModelRequestMessage {
            role: "user".to_owned(),
            content: "inspect safely".to_owned(),
        }],
        full_messages: None,
        tools: Vec::new(),
        full_tools: None,
        content_complete: true,
        applied_settings,
        applied_settings_fingerprint: None,
        truncated: false,
    }
}

/// Reads one protocol detail response for the branch's first model invocation.
///
/// - `service`: Application service holding the completed branch.
/// - `session`: Session owning the invocation.
/// - `branch`: Branch owning the invocation.
/// - `invocation`: Stable model-invocation identity.
///
/// # Returns
/// The protocol model-request detail response.
pub(super) fn dispatch_detail(
    service: &ApplicationService<Fixture>,
    session: scriberig_core::EntityId,
    branch: scriberig_core::EntityId,
    invocation: scriberig_core::EntityId,
) -> scriberig_protocol::ModelRequestDetailResponse {
    let response = Dispatcher::new(service).dispatch(RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "applied-settings-detail".to_owned(),
        request: Request::ModelRequestDetail(scriberig_protocol::ModelRequestDetailRequest {
            session: SessionReference {
                id: session.to_string(),
            },
            branch: scriberig_protocol::BranchReference {
                id: branch.to_string(),
            },
            invocation: invocation.to_string(),
            offset: None,
            max_chars: None,
        }),
    });
    let Response::ModelRequestDetail(detail) = response.response else {
        panic!("unexpected response: {:?}", response.response);
    };
    detail
}
