//! Dispatches protocol requests through the injected application boundary.

use std::str::FromStr;

use scriberig_app::{
    ApplicationContext, ApplicationError, DetailLevel, InspectRequest, SnapshotResolution,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{
    CapabilitiesResponse, CapabilityInfo, InspectEntityResponse, OperationStatusResponse,
    ProtocolError, ProtocolErrorCode, ProtocolVersion, Request, RequestEnvelope, Response,
    ResponseEnvelope, RuntimeFeature, SnapshotReference,
};

use crate::server::ApplicationService;

/// Server-side protocol dispatcher for the initial vertical slice.
pub struct Dispatcher<S> {
    service: S,
}

impl<S> Dispatcher<S> {
    /// Creates a dispatcher around an application service.
    ///
    /// # Parameters
    /// - `service`: Application implementation receiving semantic requests.
    ///
    /// # Returns
    /// A protocol dispatcher.
    #[must_use]
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S: ApplicationService> Dispatcher<S> {
    /// Dispatches one correlated protocol envelope.
    ///
    /// # Parameters
    /// - `request`: Correlated protocol request.
    ///
    /// # Returns
    /// Correlated typed response or structured error.
    #[must_use]
    pub fn dispatch(&self, request: RequestEnvelope) -> ResponseEnvelope {
        let correlation_id = request.correlation_id.clone();
        let response = if request.version.major == ProtocolVersion::default().major {
            self.dispatch_request(request.request, &correlation_id)
        } else {
            Response::Error(ProtocolError {
                code: ProtocolErrorCode::UnsupportedVersion,
                message: "unsupported protocol major version".to_owned(),
                details: Some(serde_json::json!({"major": request.version.major})),
                correlation_id: Some(correlation_id.clone()),
            })
        };
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id,
            response,
        }
    }

    /// Dispatches a version-validated typed request.
    ///
    /// # Parameters
    /// - `request`: Version-validated typed request.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// Typed protocol response.
    fn dispatch_request(&self, request: Request, correlation_id: &str) -> Response {
        match request {
            Request::Inspect(request) => self.inspect(request, correlation_id),
            Request::Capabilities(_) => {
                let capabilities = self
                    .service
                    .capabilities()
                    .into_iter()
                    .map(|capability| CapabilityInfo {
                        id: capability.id.to_string(),
                        description: capability.description,
                        available: capability.available,
                        domain: capability.scope,
                    })
                    .collect();
                Response::Capabilities(CapabilitiesResponse {
                    protocol: ProtocolVersion::default(),
                    capabilities,
                    features: vec![RuntimeFeature {
                        id: "inspect".to_owned(),
                        enabled: true,
                    }],
                })
            }
            Request::OperationStatus(request) => self
                .service
                .operation_status(&request.operation.id)
                .map_or_else(
                    |error| Response::Error(map_error(&error, correlation_id)),
                    |operation| Response::OperationStatus(OperationStatusResponse { operation }),
                ),
            Request::CancelOperation(request) => self
                .service
                .cancel_operation(&request.operation.id)
                .map_or_else(
                    |error| Response::Error(map_error(&error, correlation_id)),
                    |operation| Response::OperationCancelled(OperationStatusResponse { operation }),
                ),
            Request::Subscribe(request) => self.service.events(&request.event_kinds).map_or_else(
                |error| Response::Error(map_error(&error, correlation_id)),
                Response::Events,
            ),
        }
    }

    /// Maps one protocol inspection request to the shared application request.
    ///
    /// # Parameters
    /// - `request`: Protocol inspection request.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// Typed protocol inspection response.
    fn inspect(
        &self,
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
        let result = self.service.inspect(
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
            |error| Response::Error(map_error(&error, correlation_id)),
            |result| Response::Inspect(map_inspect(result)),
        )
    }
}

/// Converts an application inspection result to protocol references.
///
/// # Parameters
/// - `result`: Application inspection result.
///
/// # Returns
/// Protocol inspection response with stable references.
fn map_inspect(result: scriberig_app::InspectResult) -> InspectEntityResponse {
    InspectEntityResponse {
        entity: scriberig_protocol::EntityReference {
            id: result.entity.to_string(),
        },
        revision: scriberig_protocol::RevisionReference {
            entity: scriberig_protocol::EntityReference {
                id: result.entity.to_string(),
            },
            id: result
                .revision
                .map_or_else(String::new, |value| value.to_string()),
        },
        snapshot: SnapshotReference {
            id: result
                .snapshot
                .map_or_else(String::new, |value| value.to_string()),
        },
        object_type: result.object_type.map(|value| value.to_string()),
        summary: result.description.or(result.label),
    }
}

/// Converts an application error without exposing restricted payloads.
///
/// # Parameters
/// - `error`: Application error to classify.
/// - `correlation_id`: Request correlation identity.
///
/// # Returns
/// Structured protocol error.
fn map_error(error: &ApplicationError, correlation_id: &str) -> ProtocolError {
    let code = match error.category() {
        "not_found" => ProtocolErrorCode::NotFound,
        "invalid_request" => ProtocolErrorCode::InvalidRequest,
        "permission_denied" => ProtocolErrorCode::PermissionDenied,
        "schema_violation" => ProtocolErrorCode::SchemaViolation,
        "stale_base_snapshot" => ProtocolErrorCode::StaleBaseSnapshot,
        "semantic_conflict" => ProtocolErrorCode::SemanticConflict,
        "validation_failure" => ProtocolErrorCode::ValidationFailure,
        "unsupported_operation" => ProtocolErrorCode::UnsupportedOperation,
        "unsupported_capability" => ProtocolErrorCode::UnsupportedCapability,
        "conflict" => ProtocolErrorCode::Conflict,
        "ambiguous" => ProtocolErrorCode::Ambiguous,
        "snapshot_unavailable" => ProtocolErrorCode::SnapshotUnavailable,
        "operation_failed" => ProtocolErrorCode::OperationFailed,
        "projection_out_of_sync" => ProtocolErrorCode::ProjectionOutOfSync,
        "operation_cancelled" => ProtocolErrorCode::OperationCancelled,
        _ => ProtocolErrorCode::Internal,
    };
    ProtocolError {
        code,
        message: error.to_string(),
        details: Some(error_details(error)),
        correlation_id: Some(correlation_id.to_owned()),
    }
}

/// Preserves safe semantic handles in protocol error details.
///
/// # Parameters
/// - `error`: Application error being serialized.
///
/// # Returns
/// JSON-safe structured details without backend internals.
fn error_details(error: &ApplicationError) -> serde_json::Value {
    match error {
        ApplicationError::NotFound { subject } => serde_json::json!({"subject": subject}),
        ApplicationError::InvalidRequest { operation, message }
        | ApplicationError::OperationFailed { operation, message } => {
            serde_json::json!({"operation": operation, "message": message})
        }
        ApplicationError::UnsupportedCapability {
            operation,
            capability,
        } => {
            serde_json::json!({"operation": operation, "capability": capability})
        }
        ApplicationError::Conflict { subject, details } => serde_json::json!({
            "subject": subject,
            "base_revision": details.base_revision.map(|value| value.to_string()),
            "expected_revision": details.expected_revision.map(|value| value.to_string()),
            "current_revision": details.current_revision.map(|value| value.to_string()),
            "left": details.left.as_ref(),
            "right": details.right.as_ref(),
            "operation": details.operation,
        }),
        ApplicationError::Ambiguous {
            scope,
            candidates,
            reason,
        } => {
            serde_json::json!({"scope": scope, "candidates": candidates, "reason": reason})
        }
        ApplicationError::SnapshotUnavailable { snapshot } => {
            serde_json::json!({"snapshot": snapshot.to_string()})
        }
        _ => serde_json::json!({}),
    }
}
