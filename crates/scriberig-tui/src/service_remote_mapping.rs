//! Maps remote protocol values onto the TUI application boundary.

use super::{ApplicationError, EntityId};

#[path = "service_remote_activity.rs"]
mod remote_activity;

#[path = "service_remote_session_mapping.rs"]
mod session_mapping;
pub(crate) use session_mapping::{parse_entity, session_from_response};
use session_mapping::{parse_plan_parent, parse_snapshot};

#[path = "service_remote_plan_mapping.rs"]
mod plan_mapping;
pub(crate) use plan_mapping::protocol_anchor;

/// Converts exact application semantic scope to its protocol representation.
///
/// - `selection`: Validated local project, root, entity, and snapshot handles.
///
/// Provider-neutral protocol selection with stable string references.
/// # Returns
/// The equivalent protocol selection.
pub(crate) fn protocol_agent_selection(
    selection: &scriberig_app::AgentSelection,
) -> scriberig_protocol::AgentSelection {
    scriberig_protocol::AgentSelection {
        project_id: selection.project.map(|project| project.to_string()),
        root: Some(scriberig_protocol::EntityReference {
            id: selection.root.to_string(),
        }),
        entity: scriberig_protocol::EntityReference {
            id: selection.entity.to_string(),
        },
        snapshot: scriberig_protocol::SnapshotReference {
            id: selection.snapshot.to_string(),
        },
    }
}

/// Restores one complete plan projection returned by the remote service.
///
/// - `value`: Safe protocol plan projection.
///
/// Session-domain plan with validated stable identities.
///
/// Returns an invalid-request error for malformed remote identities.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn session_plan_from_response(
    value: scriberig_protocol::SessionPlan,
) -> Result<scriberig_session::Plan, ApplicationError> {
    plan_mapping::from_response(value)
}

#[path = "service_remote_operation_mapping.rs"]
mod operation_mapping;
pub(crate) use operation_mapping::map_remote_operation;
#[path = "service_remote_find.rs"]
mod remote_find;
pub(crate) use remote_find::find_entities;
#[path = "service_remote_presentation.rs"]
mod remote_presentation;
pub(crate) use remote_presentation::presentation_response;
#[path = "service_remote_event_mapping.rs"]
mod event_mapping;
pub(crate) use event_mapping::map_remote_event;

/// Lists remote snapshot entities for the TUI explorer.
///
/// - `client`: Remote protocol client.
/// - `entity`: Collection owner to exclude from the result.
/// - `snapshot`: Exact snapshot context.
/// - `continuation`: Optional page continuation.
///
/// A bounded Explorer page.
///
/// Returns a sanitized remote application error.
pub(crate) fn list_explorer<T: scriberig_protocol::ProtocolTransport>(
    client: &scriberig_client::Client<T>,
    entity: EntityId,
    snapshot: scriberig_core::SnapshotId,
    continuation: Option<scriberig_query::ContinuationToken>,
) -> Result<crate::explorer::ExplorerPage, ApplicationError> {
    let result = client
        .list(scriberig_query::ListRequest {
            subject: scriberig_query::EntityInSnapshot { entity, snapshot },
            collection: scriberig_query::CollectionRef {
                owner: entity,
                collection_type: "snapshot.entities".to_owned(),
            },
            page: scriberig_query::PageRequest {
                continuation,
                ..scriberig_query::PageRequest::default()
            },
            detail: scriberig_query::DetailLevel::Compact,
        })
        .map_err(remote_error)?;
    let rows = result
        .page
        .items
        .into_iter()
        .filter_map(|entry| match entry.target {
            scriberig_query::TargetRef::Snapshot(target) => Some(crate::explorer::ExplorerRow {
                selection: crate::selection::EntitySelection {
                    entity: target.entity,
                    revision: None,
                    snapshot: Some(target.snapshot),
                },
                label: entry.label.unwrap_or_else(|| target.entity.to_string()),
                kind: entry.entity_type.map(|value| value.to_string()),
                inspectable: true,
            }),
            scriberig_query::TargetRef::Revision(_) => None,
        })
        .filter(|row| row.selection.entity != entity)
        .collect();
    Ok(crate::explorer::ExplorerPage {
        rows,
        next: result.page.next,
    })
}

/// Finds remote entities in an exact snapshot.
///
/// - `client`: Remote protocol client.
/// - `snapshot`: Exact discovery scope.
/// - `query`: Search text.
///
/// A bounded TUI discovery model.
///
/// Returns a sanitized remote application error.
/// Converts client/protocol failures into safe application-boundary errors.
///
/// - `error`: Client or protocol failure.
///
/// A sanitized application error.
/// - `crate`: Internal visibility marker.
/// # Returns
/// The operation result.
pub(crate) fn remote_error(error: scriberig_client::ClientError) -> ApplicationError {
    match error {
        scriberig_client::ClientError::Transport(message) => {
            ApplicationError::TransportUnavailable {
                message: crate::inspector::sanitize(&message),
            }
        }
        scriberig_client::ClientError::Encoding(message) => ApplicationError::ProtocolFailure {
            category: "request_encoding".to_owned(),
            message: crate::inspector::sanitize(&message),
        },
        scriberig_client::ClientError::Protocol(error) => ApplicationError::ProtocolFailure {
            category: protocol_category(&error.code).to_owned(),
            message: remote_protocol_message(&error.message, error.details.as_ref()),
        },
        scriberig_client::ClientError::UnexpectedResponse => ApplicationError::ProtocolFailure {
            category: "unexpected_response".to_owned(),
            message: "remote response did not match the requested operation".to_owned(),
        },
    }
}

/// Adds bounded materialization receipts to a remote apply failure message.
///
/// # Parameters
/// - `message`: Original server diagnostic.
/// - `details`: Optional structured protocol error details.
///
/// # Returns
/// A terminal-safe message retaining aggregate and per-path recovery state.
pub(crate) fn remote_protocol_message(
    message: &str,
    details: Option<&serde_json::Value>,
) -> String {
    let Some(details) = details.and_then(serde_json::Value::as_object) else {
        return crate::inspector::sanitize(message);
    };
    let Some(materialization) = details
        .get("materialization")
        .and_then(serde_json::Value::as_str)
    else {
        return crate::inspector::sanitize(message);
    };
    let mut receipts = vec![format!("materialization state: {materialization}")];
    if let Some(diagnostic) = details
        .get("diagnostic")
        .and_then(serde_json::Value::as_str)
    {
        receipts.push(crate::inspector::sanitize(diagnostic));
    }
    if let Some(paths) = details.get("paths").and_then(serde_json::Value::as_array) {
        for path in paths.iter().take(16) {
            let Some(path) = path.as_object() else {
                continue;
            };
            let (Some(name), Some(state)) = (
                path.get("path").and_then(serde_json::Value::as_str),
                path.get("state").and_then(serde_json::Value::as_str),
            ) else {
                continue;
            };
            let name: String = crate::inspector::sanitize(name).chars().take(256).collect();
            let state: String = crate::inspector::sanitize(state).chars().take(64).collect();
            let diagnostic = path
                .get("diagnostic")
                .and_then(serde_json::Value::as_str)
                .map(crate::inspector::sanitize)
                .map(|value| value.chars().take(256).collect::<String>());
            receipts.push(match diagnostic {
                Some(diagnostic) => format!("{name}: {state} ({diagnostic})"),
                None => format!("{name}: {state}"),
            });
        }
        if paths.len() > 16 {
            receipts.push(format!("{} more path receipts omitted", paths.len() - 16));
        }
    }
    format!(
        "{}; {}",
        crate::inspector::sanitize(message),
        receipts.join("; ")
    )
}

/// Converts a protocol error code to the stable presentation category.
///
/// - `code`: Protocol error code to classify.
///
/// Stable snake-case category text.
/// - `crate`: Internal visibility marker.
/// # Returns
/// The operation result.
pub(crate) fn protocol_category(code: &scriberig_protocol::ProtocolErrorCode) -> &'static str {
    match code {
        scriberig_protocol::ProtocolErrorCode::InvalidRequest => "invalid_request",
        scriberig_protocol::ProtocolErrorCode::NotFound => "not_found",
        scriberig_protocol::ProtocolErrorCode::SchemaViolation => "schema_violation",
        scriberig_protocol::ProtocolErrorCode::StaleBaseSnapshot => "stale_base_snapshot",
        scriberig_protocol::ProtocolErrorCode::SemanticConflict => "semantic_conflict",
        scriberig_protocol::ProtocolErrorCode::ValidationFailure => "validation_failure",
        scriberig_protocol::ProtocolErrorCode::PermissionDenied => "permission_denied",
        scriberig_protocol::ProtocolErrorCode::UnsupportedOperation => "unsupported_operation",
        scriberig_protocol::ProtocolErrorCode::UnsupportedCapability => "unsupported_capability",
        scriberig_protocol::ProtocolErrorCode::Conflict => "conflict",
        scriberig_protocol::ProtocolErrorCode::Ambiguous => "ambiguous",
        scriberig_protocol::ProtocolErrorCode::SnapshotUnavailable => "snapshot_unavailable",
        scriberig_protocol::ProtocolErrorCode::OperationFailed => "operation_failed",
        scriberig_protocol::ProtocolErrorCode::ProjectionOutOfSync => "projection_out_of_sync",
        scriberig_protocol::ProtocolErrorCode::OperationCancelled => "operation_cancelled",
        scriberig_protocol::ProtocolErrorCode::Internal => "internal",
        scriberig_protocol::ProtocolErrorCode::UnsupportedVersion => "unsupported_version",
        scriberig_protocol::ProtocolErrorCode::CursorTooOld => "cursor_too_old",
    }
}
