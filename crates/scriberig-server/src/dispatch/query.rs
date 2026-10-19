//! Dispatches typed list, find, and query specifications.

use super::helpers;
use crate::server::ApplicationService;
use scriberig_app::ApplicationContext;
use scriberig_protocol::{ProtocolError, ProtocolErrorCode, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Executes a typed query specification.
///
/// - `service`: Application service handling semantic queries.
/// - `value`: Serialized query specification.
/// - `correlation_id`: Request correlation identity.
///
/// Query result or a structured protocol error.
pub(super) fn query<S: ApplicationService>(
    service: &S,
    value: Value,
    correlation_id: &str,
) -> Response {
    execute::<S, scriberig_query::QuerySpec>(
        service,
        value,
        correlation_id,
        "invalid query specification",
        |service, context, specification| {
            service.query(context, specification).map(|result| {
                Response::Query(scriberig_protocol::QueryResponse {
                    result: serde_json::to_value(result).unwrap_or_default(),
                })
            })
        },
    )
}

/// Executes a typed list specification.
///
/// - `service`: Application service handling semantic lists.
/// - `value`: Serialized list specification.
/// - `correlation_id`: Request correlation identity.
///
/// List result or a structured protocol error.
pub(super) fn list<S: ApplicationService>(
    service: &S,
    value: Value,
    correlation_id: &str,
) -> Response {
    execute::<S, scriberig_query::ListRequest>(
        service,
        value,
        correlation_id,
        "invalid list specification",
        |service, context, specification| {
            service.list(context, specification).map(|result| {
                Response::List(scriberig_protocol::ListResponse {
                    result: serde_json::to_value(result).unwrap_or_default(),
                })
            })
        },
    )
}

/// Executes a typed find specification.
///
/// - `service`: Application service handling semantic discovery.
/// - `value`: Serialized find specification.
/// - `correlation_id`: Request correlation identity.
///
/// Find result or a structured protocol error.
pub(super) fn find<S: ApplicationService>(
    service: &S,
    value: Value,
    correlation_id: &str,
) -> Response {
    execute::<S, scriberig_query::FindRequest>(
        service,
        value,
        correlation_id,
        "invalid find specification",
        |service, context, specification| {
            service.find(context, specification).map(|result| {
                Response::Find(scriberig_protocol::FindResponse {
                    result: serde_json::to_value(result).unwrap_or_default(),
                })
            })
        },
    )
}

/// Deserializes and executes one typed semantic specification.
///
/// - `service`: Application service receiving the typed operation.
/// - `value`: Serialized semantic specification.
/// - `correlation_id`: Request correlation identity.
/// - `invalid_message`: Safe message for malformed input.
/// - `operation`: Typed application operation to invoke.
///
/// Operation response or a structured schema/application error.
fn execute<S, T>(
    service: &S,
    value: Value,
    correlation_id: &str,
    invalid_message: &'static str,
    operation: impl FnOnce(
        &S,
        &ApplicationContext,
        &T,
    ) -> Result<Response, scriberig_app::ApplicationError>,
) -> Response
where
    S: ApplicationService,
    T: DeserializeOwned,
{
    let Ok(specification) = serde_json::from_value::<T>(value) else {
        return Response::Error(ProtocolError {
            code: ProtocolErrorCode::SchemaViolation,
            message: invalid_message.to_owned(),
            details: None,
            correlation_id: Some(correlation_id.to_owned()),
        });
    };
    operation(service, &ApplicationContext::empty(), &specification)
        .unwrap_or_else(|error| Response::Error(helpers::map_error(&error, correlation_id)))
}
