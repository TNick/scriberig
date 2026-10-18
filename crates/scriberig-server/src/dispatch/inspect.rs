//! Maps protocol inspection requests through the application boundary.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, DetailLevel, InspectRequest, SnapshotResolution};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{ProtocolError, ProtocolErrorCode, Response};

use crate::server::ApplicationService;

/// Inspects one semantic entity and maps the result to a protocol response.
///
/// # Parameters
/// - `service`: Application service receiving the inspection.
/// - `request`: Typed protocol inspection request.
/// - `correlation_id`: Request correlation identity.
///
/// # Returns
/// Typed inspection response or structured protocol error.
pub(super) fn inspect<S: ApplicationService>(
    service: &S,
    request: scriberig_protocol::InspectEntityRequest,
    correlation_id: &str,
) -> Response {
    let Ok(entity) = EntityId::from_str(&request.entity.id) else {
        return Response::Error(ProtocolError {
            code: ProtocolErrorCode::SchemaViolation,
            message: "invalid entity identifier".to_owned(),
            details: None,
            correlation_id: Some(correlation_id.to_owned()),
        });
    };
    let snapshot = match request.snapshot {
        scriberig_protocol::SnapshotSelector::Current
        | scriberig_protocol::SnapshotSelector::NamedRoot(_) => SnapshotResolution::Current,
        scriberig_protocol::SnapshotSelector::Exact(reference) => {
            match SnapshotId::from_str(&reference.id) {
                Ok(value) => SnapshotResolution::At(value),
                Err(_) => {
                    return Response::Error(ProtocolError {
                        code: ProtocolErrorCode::SchemaViolation,
                        message: "invalid snapshot identifier".to_owned(),
                        details: None,
                        correlation_id: Some(correlation_id.to_owned()),
                    });
                }
            }
        }
    };
    let result = service.inspect(
        &ApplicationContext::empty(),
        &InspectRequest {
            entity,
            snapshot,
            detail: match request.detail {
                scriberig_protocol::request::DetailLevel::Compact => DetailLevel::Compact,
                scriberig_protocol::request::DetailLevel::Deep => DetailLevel::Detailed,
            },
        },
    );
    result.map_or_else(
        |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
        |result| Response::Inspect(super::helpers::map_inspect(result)),
    )
}
