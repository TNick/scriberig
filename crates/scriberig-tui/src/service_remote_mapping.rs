//! Maps remote protocol values onto the TUI application boundary.

use std::str::FromStr;

use super::{ApplicationError, EntityId};

#[path = "service_remote_operation_mapping.rs"]
mod operation_mapping;
pub(crate) use operation_mapping::map_remote_operation;

/// Lists remote snapshot entities for the TUI explorer.
///
/// # Parameters
/// - `client`: Remote protocol client.
/// - `entity`: Collection owner to exclude from the result.
/// - `snapshot`: Exact snapshot context.
/// - `continuation`: Optional page continuation.
///
/// # Returns
/// A bounded Explorer page.
///
/// # Errors
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
/// # Parameters
/// - `client`: Remote protocol client.
/// - `snapshot`: Exact discovery scope.
/// - `query`: Search text.
///
/// # Returns
/// A bounded TUI discovery model.
///
/// # Errors
/// Returns a sanitized remote application error.
pub(crate) fn find_entities<T: scriberig_protocol::ProtocolTransport>(
    client: &scriberig_client::Client<T>,
    snapshot: scriberig_core::SnapshotId,
    query: &str,
) -> Result<crate::view::find::FindModel, ApplicationError> {
    let result = client
        .find(scriberig_query::FindRequest {
            scope: scriberig_query::QueryScope::Snapshot(snapshot),
            query: query.to_owned(),
            strategy: scriberig_query::FindStrategy::ExactText,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Compact,
        })
        .map_err(remote_error)?;
    Ok(crate::view::find::FindModel::from_result(query, &result))
}

/// Converts a compact protocol session response to the shared session model.
///
/// # Parameters
/// - `response`: Protocol session tree response.
///
/// # Returns
/// Session value accepted by the existing TUI projection.
///
/// # Errors
/// Returns an internal error when a remote reference is malformed.
/// - `crate`: Internal visibility marker.
pub(crate) fn session_from_response(
    response: scriberig_protocol::SessionReadResponse,
) -> Result<scriberig_session::Session, ApplicationError> {
    let id = parse_entity(&response.session.id)?;
    let branches = response
        .branches
        .into_iter()
        .map(|value| {
            let branch_id = parse_entity(&value.id)?;
            let mut branch =
                scriberig_session::Branch::new(branch_id, value.title).map_err(|error| {
                    ApplicationError::Internal {
                        message: format!("remote branch was invalid: {error}"),
                    }
                })?;
            branch.parent = value.parent.as_deref().map(parse_entity).transpose()?;
            branch.children = value
                .children
                .iter()
                .map(|child| parse_entity(child))
                .collect::<Result<Vec<_>, _>>()?;
            let status = match value.status {
                scriberig_protocol::SessionBranchStatus::Active => {
                    scriberig_session::BranchStatus::Active
                }
                scriberig_protocol::SessionBranchStatus::Completed => {
                    scriberig_session::BranchStatus::Completed
                }
                scriberig_protocol::SessionBranchStatus::Abandoned => {
                    scriberig_session::BranchStatus::Abandoned
                }
            };
            branch
                .transition(status)
                .map_err(|error| ApplicationError::Internal {
                    message: format!("remote branch status was invalid: {error}"),
                })?;
            branch.turns = value
                .turns
                .into_iter()
                .map(|turn| {
                    Ok(scriberig_session::Turn {
                        id: parse_entity(&turn.id)?,
                        ordinal: turn.ordinal,
                        human: turn.has_human_input.then(|| scriberig_session::HumanInput {
                            text: String::new(),
                        }),
                        agent: turn
                            .has_agent_output
                            .then(|| scriberig_session::AgentOutput {
                                text: turn.agent_output.unwrap_or_default(),
                            }),
                        tool_invocation: turn
                            .tool_invocation
                            .as_deref()
                            .map(parse_entity)
                            .transpose()?,
                        tool_result: turn.tool_result.as_deref().map(parse_entity).transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, ApplicationError>>()?;
            branch.result = value
                .result
                .map(|result| {
                    Ok(scriberig_session::BranchResult {
                        id: parse_entity(&result.id)?,
                        branch: branch_id,
                        status: scriberig_session::BranchStatus::Completed,
                        summary: None,
                        result_kind: None,
                        references: result
                            .references
                            .iter()
                            .map(|reference| parse_entity(reference))
                            .collect::<Result<Vec<_>, _>>()?,
                        touched: Vec::new(),
                        produced_changes: Vec::new(),
                        validations: Vec::new(),
                        unresolved_questions: Vec::new(),
                    })
                })
                .transpose()?;
            Ok(branch)
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    Ok(scriberig_session::Session {
        id,
        branches,
        goals: Vec::new(),
        tasks: Vec::new(),
    })
}

/// Parses one remote entity identity and sanitizes malformed responses.
///
/// # Parameters
/// - `value`: Remote entity identity text.
///
/// # Returns
/// Parsed entity identity.
///
/// # Errors
/// Returns an internal error for malformed remote identity text.
/// - `crate`: Internal visibility marker.
pub(crate) fn parse_entity(value: &str) -> Result<EntityId, ApplicationError> {
    EntityId::from_str(value).map_err(|error| ApplicationError::Internal {
        message: format!("remote response contained invalid entity identity: {error}"),
    })
}

/// Parses one remote snapshot identity and sanitizes malformed responses.
///
/// # Parameters
/// - `value`: Remote snapshot identity text.
///
/// # Returns
/// Parsed snapshot identity.
///
/// # Errors
/// Returns an internal error for malformed remote identity text.
/// - `super`: Internal visibility marker.
pub(super) fn parse_snapshot(value: &str) -> Result<scriberig_core::SnapshotId, ApplicationError> {
    scriberig_core::SnapshotId::from_str(value).map_err(|error| ApplicationError::Internal {
        message: format!("remote response contained invalid snapshot identity: {error}"),
    })
}

/// Converts operation-related protocol events into shared application events.
/// - `event`: Argument named `event`.
///
/// # Returns
/// The value produced by the declaration.
/// - `crate`: Internal visibility marker.
pub(crate) fn map_remote_event(
    event: scriberig_protocol::Event,
) -> Option<scriberig_app::ApplicationEvent> {
    let operation = event.operation?.id.parse().ok()?;
    match event.kind {
        scriberig_protocol::EventKind::OperationStarted => {
            Some(scriberig_app::ApplicationEvent::OperationStarted { operation })
        }
        scriberig_protocol::EventKind::OperationProgressed => {
            let progress = event
                .metadata
                .iter()
                .find(|(key, _)| key == "percent")
                .and_then(|(_, value)| value.parse::<u8>().ok())
                .map(|percent| scriberig_app::OperationProgress {
                    status: scriberig_app::OperationStatus::Running,
                    completed: Some(u64::from(percent)),
                    total: Some(100),
                    phase: None,
                })?;
            Some(scriberig_app::ApplicationEvent::OperationProgressed {
                operation,
                progress,
            })
        }
        scriberig_protocol::EventKind::OperationCompleted => {
            Some(scriberig_app::ApplicationEvent::OperationCompleted { operation })
        }
        scriberig_protocol::EventKind::OperationFailed => {
            Some(scriberig_app::ApplicationEvent::OperationFailed {
                operation,
                error: event
                    .metadata
                    .iter()
                    .find(|(key, _)| key == "error")
                    .map(|(_, value)| value.clone())
                    .unwrap_or_else(|| "operation failed".to_owned()),
            })
        }
        scriberig_protocol::EventKind::OperationCancelled => {
            Some(scriberig_app::ApplicationEvent::OperationCancelled { operation })
        }
        scriberig_protocol::EventKind::AgentOutputProduced => {
            let output = event
                .metadata
                .iter()
                .find(|(key, _)| key == "output")
                .map(|(_, value)| value.clone())?;
            Some(scriberig_app::ApplicationEvent::AgentOutputProduced { operation, output })
        }
        scriberig_protocol::EventKind::AgentOutputDelta => {
            let text = event
                .metadata
                .iter()
                .find(|(key, _)| key == "text")
                .map(|(_, value)| value.clone())?;
            Some(scriberig_app::ApplicationEvent::AgentOutputDelta { operation, text })
        }
        _ => None,
    }
}

/// Converts client/protocol failures into safe application-boundary errors.
///
/// # Parameters
/// - `error`: Client or protocol failure.
///
/// # Returns
/// A sanitized application error.
/// - `crate`: Internal visibility marker.
pub(crate) fn remote_error(error: scriberig_client::ClientError) -> ApplicationError {
    match error {
        scriberig_client::ClientError::Transport(message) => {
            ApplicationError::TransportUnavailable {
                message: crate::inspector::sanitize(&message),
            }
        }
        scriberig_client::ClientError::Protocol(error) => ApplicationError::ProtocolFailure {
            category: protocol_category(&error.code).to_owned(),
            message: crate::inspector::sanitize(&error.message),
        },
        scriberig_client::ClientError::UnexpectedResponse => ApplicationError::ProtocolFailure {
            category: "unexpected_response".to_owned(),
            message: "remote response did not match the requested operation".to_owned(),
        },
    }
}

/// Converts a protocol error code to the stable presentation category.
///
/// # Parameters
/// - `code`: Protocol error code to classify.
///
/// # Returns
/// Stable snake-case category text.
/// - `crate`: Internal visibility marker.
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
