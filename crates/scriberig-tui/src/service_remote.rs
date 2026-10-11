//! Adapts remote protocol operations to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Adapts a protocol transport to TUI service operations.
pub struct RemoteAdapter<T> {
    /// Protocol client used for semantic requests.
    client: scriberig_client::Client<T>,
}

impl<T> RemoteAdapter<T> {
    /// Creates a remote TUI adapter over a public protocol client.
    #[must_use]
    pub const fn new(client: scriberig_client::Client<T>) -> Self {
        Self { client }
    }
}

impl<T: scriberig_protocol::ProtocolTransport> TuiService for RemoteAdapter<T> {
    /// Inspects an entity through the public protocol client.
    ///
    /// # Parameters
    /// - `_context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The decoded inspection result.
    ///
    /// # Errors
    /// Returns a protocol or response-decoding error.
    fn inspect_entity(
        &self,
        _context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        let selector = snapshot.map_or(scriberig_protocol::SnapshotSelector::Current, |value| {
            scriberig_protocol::SnapshotSelector::Exact(scriberig_protocol::SnapshotReference {
                id: value.to_string(),
            })
        });
        let response = self
            .client
            .inspect(entity.to_string(), selector, true)
            .map_err(remote_error)?;
        let resolved_entity = EntityId::from_str(&response.entity.id).map_err(|error| {
            ApplicationError::Internal {
                message: format!("remote response contained invalid entity identity: {error}"),
            }
        })?;
        let resolved_snapshot = SnapshotId::from_str(&response.snapshot.id).map_err(|error| {
            ApplicationError::Internal {
                message: format!("remote response contained invalid snapshot identity: {error}"),
            }
        })?;
        let revision = RevisionId::from_str(&response.revision.id).ok();
        Ok(InspectResult {
            entity: resolved_entity,
            revision,
            snapshot: Some(resolved_snapshot),
            object_type: response
                .object_type
                .and_then(|value| scriberig_core::ObjectType::new(value).ok()),
            label: response.summary,
            description: None,
            capabilities: Vec::new(),
            trust: None,
        })
    }

    /// Converts remote capability descriptors into application metadata.
    ///
    /// # Parameters
    /// - `_context`: Application context for capability access.
    ///
    /// # Returns
    /// Decoded runtime capabilities.
    ///
    /// # Errors
    /// Returns a protocol capability-discovery error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        self.client
            .capabilities()
            .map_err(remote_error)
            .map(|response| {
                response
                    .capabilities
                    .into_iter()
                    .filter_map(|capability| {
                        Some(RuntimeCapability {
                            id: scriberig_app::CapabilityId::new(capability.id).ok()?,
                            description: capability.description,
                            available: capability.available,
                            scope: capability.domain,
                        })
                    })
                    .collect()
            })
    }

    /// Reads a remote operation lifecycle record through the public client.
    ///
    /// # Parameters
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The remote operation record.
    ///
    /// # Errors
    /// Returns a remote protocol or decoding error.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        let response = self
            .client
            .operation_status(operation.to_string())
            .map_err(remote_error)?;
        map_remote_operation(response.operation)
    }

    /// Requests remote cancellation and returns the reported lifecycle state.
    ///
    /// # Parameters
    /// - `_context`: Application context for cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled remote operation record.
    ///
    /// # Errors
    /// Returns a remote protocol or decoding error.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        let response = self
            .client
            .cancel_operation(operation.to_string())
            .map_err(remote_error)?;
        map_remote_operation(response.operation)
    }

    /// Polls public remote events for the runtime event/action path.
    ///
    /// # Parameters
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Decoded remote application events.
    ///
    /// # Errors
    /// Returns a remote protocol error.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        self.client
            .subscribe(Vec::new())
            .map_err(remote_error)
            .map(|events| events.into_iter().filter_map(map_remote_event).collect())
    }
}

/// Converts one protocol operation into the shared application lifecycle model.
/// - `operation`: Argument named `operation`.
///
/// # Returns
/// The value produced by the declaration.
///
/// # Errors
/// Returns an error when the operation cannot complete.
fn map_remote_operation(
    operation: scriberig_protocol::Operation,
) -> Result<scriberig_app::OperationRecord, ApplicationError> {
    let reference = scriberig_core::OperationId::from_str(&operation.reference.id)
        .or_else(|_| scriberig_core::OperationId::new(operation.reference.id.clone()))
        .map_err(|error| ApplicationError::Internal {
            message: format!("remote response contained invalid operation identity: {error}"),
        })?;
    let kind = match operation.kind {
        scriberig_protocol::operation::OperationKind::Inspect => {
            scriberig_app::OperationKind::Inspect
        }
        scriberig_protocol::operation::OperationKind::Query => scriberig_app::OperationKind::Query,
        scriberig_protocol::operation::OperationKind::List => scriberig_app::OperationKind::List,
        scriberig_protocol::operation::OperationKind::Follow => {
            scriberig_app::OperationKind::Follow
        }
        scriberig_protocol::operation::OperationKind::Find => scriberig_app::OperationKind::Find,
        scriberig_protocol::operation::OperationKind::Diff => scriberig_app::OperationKind::Diff,
        scriberig_protocol::operation::OperationKind::History => {
            scriberig_app::OperationKind::History
        }
        scriberig_protocol::operation::OperationKind::Render => {
            scriberig_app::OperationKind::Render
        }
        scriberig_protocol::operation::OperationKind::Branch => {
            scriberig_app::OperationKind::Branch
        }
        scriberig_protocol::operation::OperationKind::Apply => scriberig_app::OperationKind::Apply,
        scriberig_protocol::operation::OperationKind::Commit => {
            scriberig_app::OperationKind::Commit
        }
        scriberig_protocol::operation::OperationKind::Validate
        | scriberig_protocol::operation::OperationKind::Promote
        | scriberig_protocol::operation::OperationKind::Custom(_) => {
            scriberig_app::OperationKind::Query
        }
    };
    let status = match operation.status {
        scriberig_protocol::OperationStatus::Queued => scriberig_app::OperationStatus::Queued,
        scriberig_protocol::OperationStatus::Running => scriberig_app::OperationStatus::Running,
        scriberig_protocol::OperationStatus::Completed => scriberig_app::OperationStatus::Completed,
        scriberig_protocol::OperationStatus::Failed => scriberig_app::OperationStatus::Failed,
        scriberig_protocol::OperationStatus::Cancelled => scriberig_app::OperationStatus::Cancelled,
    };
    let mut record = scriberig_app::OperationRecord::queued(reference, kind);
    record.status = status;
    record.result_handles = operation.result_handles;
    record.progress = operation
        .progress
        .map(|progress| scriberig_app::OperationProgress {
            status,
            completed: Some(u64::from(progress.percent)),
            total: Some(100),
            phase: progress.message,
        });
    if let Some(error) = operation.error {
        record.error = Some(ApplicationError::OperationFailed {
            operation: record.reference.to_string(),
            message: crate::inspector::sanitize(&error),
        });
    }
    Ok(record)
}

/// Converts operation-related protocol events into shared application events.
/// - `event`: Argument named `event`.
///
/// # Returns
/// The value produced by the declaration.
fn map_remote_event(event: scriberig_protocol::Event) -> Option<scriberig_app::ApplicationEvent> {
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
fn remote_error(error: scriberig_client::ClientError) -> ApplicationError {
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
fn protocol_category(code: &scriberig_protocol::ProtocolErrorCode) -> &'static str {
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
    }
}
