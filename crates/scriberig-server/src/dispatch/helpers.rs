//! Shared protocol conversion helpers for the server dispatcher.

use scriberig_app::ApplicationError;
use scriberig_protocol::{
    EntityReference, InspectEntityResponse, ProtocolError, ProtocolErrorCode, RevisionReference,
    SessionReadResponse, SnapshotReference,
};

use crate::server::server_helpers::error_details;

/// Restricts a session response to one branch and its descendants.
///
/// # Parameters
/// - `response`: Durable session payload before filtering.
/// - `branch`: Branch identifier to retain in the projection.
///
/// # Returns
/// The original session response reduced to the selected branch subtree.
pub fn select_branch_subtree(response: SessionReadResponse, branch: &str) -> SessionReadResponse {
    let mut selected = std::collections::BTreeSet::from([branch.to_owned()]);
    let mut changed = true;
    while changed {
        changed = false;
        for candidate in &response.branches {
            if candidate
                .parent
                .as_ref()
                .is_some_and(|parent| selected.contains(parent))
                && selected.insert(candidate.id.clone())
            {
                changed = true;
            }
        }
    }

    let branches = response
        .branches
        .into_iter()
        .filter(|candidate| selected.contains(&candidate.id))
        .collect();

    SessionReadResponse {
        branches,
        ..response
    }
}

/// Converts an application inspection result to protocol references.
///
/// # Parameters
/// - `result`: Application inspection result.
///
/// # Returns
/// Protocol inspection response with stable references.
pub fn map_inspect(result: scriberig_app::InspectResult) -> InspectEntityResponse {
    InspectEntityResponse {
        entity: EntityReference {
            id: result.entity.to_string(),
        },
        revision: RevisionReference {
            entity: EntityReference {
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
pub fn map_error(error: &ApplicationError, correlation_id: &str) -> ProtocolError {
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
        "cursor_too_old" => ProtocolErrorCode::CursorTooOld,
        _ => ProtocolErrorCode::Internal,
    };
    ProtocolError {
        code,
        message: error.to_string(),
        details: Some(error_details(error)),
        correlation_id: Some(correlation_id.to_owned()),
    }
}
