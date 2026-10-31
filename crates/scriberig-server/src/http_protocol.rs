//! Maps operation-segment HTTP requests onto the typed protocol dispatcher.

use serde_json::{Value, json};

use crate::dispatch::Dispatcher;
use crate::http::HttpState;
use crate::server::ApplicationService;

/// Dispatches one operation-segment request through the public protocol.
///
/// - state: Shared application service and fixture state.
/// - operation: Stable operation identifier from the URL.
/// - body: JSON request payload.
/// - `correlation_id`: Request correlation identity.
///
/// A serialized typed protocol response envelope.
///
/// Returns a transport status and structured body for malformed or unknown
/// operations.
pub(super) fn dispatch<S: ApplicationService + 'static>(
    state: &HttpState<S>,
    operation: &str,
    body: &str,
    correlation_id: String,
) -> Result<Value, (u16, Value)> {
    let request = decode_request(operation, body)?;
    if let scriberig_protocol::Request::Project(project) = request {
        return serde_json::to_value(super::project_routes::protocol(
            state,
            &project,
            correlation_id,
        ))
        .map_err(|error| {
            (
                500,
                json!({"code": "internal", "message": error.to_string()}),
            )
        });
    }
    let is_model_library = matches!(&request, scriberig_protocol::Request::ModelLibrary(_));
    // Read the generation before dispatch: upstream catalog revisions only
    // advance, so a readiness projection aligned to this value proves no reload
    // replaced the model rows while this response was being assembled.
    let observed_generation = is_model_library
        .then(|| state.service.model_catalog_generation())
        .flatten();
    let lifecycle_handler = state.plan_lifecycle_handler.as_ref().map(|handler| {
        let handler = std::sync::Arc::clone(handler);
        std::sync::Arc::new(
            move |service: &&S,
                  context: &scriberig_app::ApplicationContext,
                  request: &scriberig_app::PlanLifecycleRequest,
                  action: scriberig_app::PlanLifecycleAction| {
                handler(*service, context, request, action)
            },
        ) as std::sync::Arc<crate::http::PlanLifecycleHandler<&S>>
    });
    let agent_handler = state.agent_execution_handler.as_ref().map(|handler| {
        let handler = std::sync::Arc::clone(handler);
        std::sync::Arc::new(
            move |service: &&S,
                  context: &scriberig_app::ApplicationContext,
                  request: &scriberig_app::AgentCommandRequest,
                  accepted: Option<&scriberig_protocol::AgentAcceptedResponse>| {
                handler(*service, context, request, accepted)
            },
        ) as std::sync::Arc<crate::http::AgentExecutionHandler<&S>>
    });
    let capabilities = trusted_project_capabilities(state, &request);
    let mut response = Dispatcher::for_principal(&state.service, state.principal.clone())
        .with_capabilities(capabilities)
        .dispatch_with_runtime_handlers(
            scriberig_protocol::RequestEnvelope {
                version: scriberig_protocol::ProtocolVersion::default(),
                correlation_id,
                request,
            },
            agent_handler.as_deref(),
            lifecycle_handler.as_deref(),
        );
    if is_model_library
        && let (Some(handler), scriberig_protocol::Response::ModelLibrary(library)) = (
            state.provider_readiness_handler.as_ref(),
            &mut response.response,
        )
    {
        super::http_readiness::attach_provider_readiness(library, handler(), observed_generation);
    }
    serde_json::to_value(response).map_err(|error| {
        (
            500,
            json!({"code": "internal", "message": error.to_string()}),
        )
    })
}

/// Grants local Inspect only when the requested project exists in the host catalog.
///
/// The HTTP server is a single-user local transport: callers cannot submit grants,
/// and project access is resolved from the host-owned SQLite catalog.
///
/// - state: Trusted local server state.
/// - request: Typed request whose project scope may require Inspect.
///
/// Host-authorized project capability context, empty for all other cases.
fn trusted_project_capabilities<S>(
    state: &HttpState<S>,
    request: &scriberig_protocol::Request,
) -> scriberig_app::CapabilityContext {
    let project = match request {
        scriberig_protocol::Request::ArtifactInspect(request) => request.project.parse().ok(),
        scriberig_protocol::Request::ArtifactMembers(request) => request.project.parse().ok(),
        scriberig_protocol::Request::CadDrawingInspect(request) => request.project.parse().ok(),
        scriberig_protocol::Request::CadEntities(request) => request.project.parse().ok(),
        scriberig_protocol::Request::CadBoundsQuery(request) => request.project.parse().ok(),
        scriberig_protocol::Request::CadLineEdit(request) => request.project.parse().ok(),
        scriberig_protocol::Request::CadLayerChange(request) => request.project.parse().ok(),
        scriberig_protocol::Request::SpreadsheetCells(request) => request.project.parse().ok(),
        scriberig_protocol::Request::SpreadsheetCellInspect(request) => {
            request.project.parse().ok()
        }
        scriberig_protocol::Request::SpreadsheetWorkbook(request) => request.project.parse().ok(),
        scriberig_protocol::Request::SpreadsheetCellFind(request) => request.project.parse().ok(),
        scriberig_protocol::Request::SpreadsheetEdit(request) => request.project.parse().ok(),
        scriberig_protocol::Request::SpreadsheetRangeEdit(request) => request.project.parse().ok(),
        scriberig_protocol::Request::SpreadsheetRename(request) => request.project.parse().ok(),
        scriberig_protocol::Request::DocumentParagraphEdit(request) => request.project.parse().ok(),
        scriberig_protocol::Request::DocumentTableCellEdit(request) => request.project.parse().ok(),
        scriberig_protocol::Request::DocumentParagraphInsert(request) => {
            request.project.parse().ok()
        }
        scriberig_protocol::Request::DocumentInspect(request) => request.project.parse().ok(),
        scriberig_protocol::Request::DocumentParagraphFind(request) => request.project.parse().ok(),
        scriberig_protocol::Request::GeoFeatures(request) => request.project.parse().ok(),
        scriberig_protocol::Request::GeoFeatureQuery(request) => request.project.parse().ok(),
        scriberig_protocol::Request::QgisProject(request) => request.project.parse().ok(),
        scriberig_protocol::Request::QgisLayerRename(request) => request.project.parse().ok(),
        scriberig_protocol::Request::QgisLayerOrder(request) => request.project.parse().ok(),
        scriberig_protocol::Request::GeoGeometryEdit(request) => request.project.parse().ok(),
        scriberig_protocol::Request::GeoAttributeEdit(request) => request.project.parse().ok(),
        scriberig_protocol::Request::GeoCatalog(request) => request.project.parse().ok(),
        _ => None,
    };
    let Some(project) = project else {
        return scriberig_app::CapabilityContext::default();
    };
    let Some(store) = &state.project_store else {
        return scriberig_app::CapabilityContext::default();
    };
    if !matches!(store.project(project), Ok(Some(_))) {
        return scriberig_app::CapabilityContext::default();
    }
    let actions = match request {
        scriberig_protocol::Request::SpreadsheetEdit(_)
        | scriberig_protocol::Request::SpreadsheetRangeEdit(_)
        | scriberig_protocol::Request::SpreadsheetRename(_)
        | scriberig_protocol::Request::DocumentParagraphEdit(_)
        | scriberig_protocol::Request::DocumentTableCellEdit(_)
        | scriberig_protocol::Request::DocumentParagraphInsert(_)
        | scriberig_protocol::Request::GeoGeometryEdit(_)
        | scriberig_protocol::Request::GeoAttributeEdit(_)
        | scriberig_protocol::Request::QgisLayerRename(_)
        | scriberig_protocol::Request::QgisLayerOrder(_)
        | scriberig_protocol::Request::CadLineEdit(_)
        | scriberig_protocol::Request::CadLayerChange(_) => {
            std::collections::BTreeSet::from([scriberig_app::CapabilityAction::Propose])
        }
        _ => std::collections::BTreeSet::from([scriberig_app::CapabilityAction::Inspect]),
    };
    scriberig_app::CapabilityContext::with_grants([scriberig_app::Capability {
        scope: scriberig_app::CapabilityScope::Custom(format!("project:{project}")),
        actions,
        constraints: scriberig_app::CapabilityConstraints::unrestricted(),
    }])
}

#[path = "http_protocol_decode.rs"]
mod request_decoder;
use request_decoder::decode_request;
