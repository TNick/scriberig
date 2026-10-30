//! Dispatches shared projection style operations through the application service.

use crate::server::ApplicationService;
use scriberig_app::ApplicationContext;
use scriberig_protocol::{
    ProjectionStyleResolveRequest, ProjectionStyleSchemaRequest, ProjectionStyleSettingRequest,
    Response,
};

/// Reads one adapter-owned style schema.
///
/// - `service`: Shared application service.
/// - `context`: Trusted request context.
/// - `request`: Projection kind whose schema is requested.
/// - `correlation_id`: Request tracing identity.
///
/// The typed schema response or protocol error.
///
/// # Returns
/// The schema response.
pub(super) fn schema<S: ApplicationService>(
    service: &S,
    context: &ApplicationContext,
    request: &ProjectionStyleSchemaRequest,
    correlation_id: &str,
) -> Response {
    service
        .projection_style_schema(context, &request.projection_kind)
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            Response::ProjectionStyleSchema,
        )
}

/// Resolves purpose-aware style through the shared application service.
///
/// - `service`: Shared application service.
/// - `context`: Trusted principal and request context.
/// - `request`: Style purpose and scope selection.
/// - `correlation_id`: Request tracing identity.
///
/// The resolved style, provenance, and fidelity response.
///
/// # Returns
/// The typed style response or protocol error.
pub(super) fn resolve<S: ApplicationService>(
    service: &S,
    context: &ApplicationContext,
    request: &ProjectionStyleResolveRequest,
    correlation_id: &str,
) -> Response {
    service
        .resolve_projection_style(context, request)
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            Response::ProjectionStyleResolve,
        )
}

/// Sets or clears style in one explicit configuration scope.
///
/// - `service`: Shared application service.
/// - `context`: Trusted principal and authorization context.
/// - `request`: Setting scope, style bytes, and revision precondition.
/// - `correlation_id`: Request tracing identity.
///
/// The authoritative setting result.
///
/// # Returns
/// The typed mutation response or protocol error.
pub(super) fn set<S: ApplicationService>(
    service: &S,
    context: &ApplicationContext,
    request: &ProjectionStyleSettingRequest,
    correlation_id: &str,
) -> Response {
    service.set_projection_style(context, request).map_or_else(
        |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
        Response::ProjectionStyleSetting,
    )
}
