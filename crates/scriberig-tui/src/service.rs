//! Defines the narrow application-facing port used by TUI semantic views.

use crate::{action::Action, effect::Effect, inspector::InspectorModel};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, InspectRequest, InspectResult, QueryService,
    RuntimeCapability, SnapshotResolution,
};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use std::str::FromStr;

/// Supplies semantic reads to the TUI without exposing storage internals.
pub trait TuiService {
    /// Inspects one entity through the shared application contract.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The application inspection result.
    ///
    /// # Errors
    /// Returns the structured application error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError>;

    /// Returns runtime capabilities exposed by the application composition.
    ///
    /// # Parameters
    /// - `_context`: Application context for capability resolution.
    ///
    /// # Returns
    /// The capabilities available to the TUI.
    ///
    /// # Errors
    /// Returns the structured application error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(Vec::new())
    }

    /// Executes one shared application operation when the adapter supports it.
    ///
    /// # Parameters
    /// - `_context`: Application context for the operation.
    /// - `_request`: Public operation request.
    ///
    /// # Returns
    /// The operation result when supported.
    ///
    /// # Errors
    /// Returns a structured application error when the operation is unavailable
    /// or rejected by the owning application subsystem.
    fn execute_operation(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "tui semantic operation".to_owned(),
        })
    }

    /// Reads one authoritative operation lifecycle record.
    ///
    /// # Parameters
    /// - `_context`: Application context for the lookup.
    /// - `_operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The current operation record.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    /// - `_context`: Argument named `_context`.
    /// - `operation`: Argument named `operation`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        _operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation status".to_owned(),
        })
    }

    /// Requests application-owned cancellation of one operation.
    ///
    /// # Parameters
    /// - `_context`: Application context for the cancellation.
    /// - `_operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled operation record.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    /// - `_context`: Argument named `_context`.
    /// - `operation`: Argument named `operation`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        _operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation cancellation".to_owned(),
        })
    }

    /// Returns application events observed since the previous runtime poll.
    ///
    /// # Parameters
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Retained application events.
    ///
    /// # Errors
    /// Returns an unsupported event-access error when unavailable.
    /// - `_context`: Argument named `_context`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        Ok(Vec::new())
    }
}

/// Supplies shared application operations to semantic TUI views.
pub trait TuiOperationService {
    /// Executes one public application operation without exposing storage.
    ///
    /// # Parameters
    /// - `context`: Authorized application context.
    /// - `request`: Public universal application request.
    ///
    /// # Returns
    /// Structured application result owned by the application layer.
    ///
    /// # Errors
    /// Returns the structured application error from the owning subsystem.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError>;
}

/// Adapts the complete public application facade for operation-backed views.
pub struct ApplicationOperationAdapter<B> {
    service: ApplicationService<B>,
}

/// Adapts the public protocol client to the TUI inspection boundary.
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
    ApplicationError::Internal {
        message: crate::inspector::sanitize(&error.to_string()),
    }
}

impl<B> ApplicationOperationAdapter<B> {
    /// Creates an operation adapter around an application service.
    #[must_use]
    pub const fn new(service: ApplicationService<B>) -> Self {
        Self { service }
    }
}

impl<B: ApplicationBackend> TuiService for ApplicationOperationAdapter<B> {
    /// Delegates inspection through the public application facade.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the application inspection error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.service.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }

    /// Delegates shared operations through the public application facade.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Public operation request.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns the application operation error.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        self.service.execute(context, request)
    }

    /// Reads an operation record from the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The current operation record.
    ///
    /// # Errors
    /// Returns a lookup error when the operation is unknown.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        self.service.operations().get(operation)
    }

    /// Cancels an operation through the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for the cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled operation record.
    ///
    /// # Errors
    /// Returns a cancellation error when the operation is unknown.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        self.service.operations().cancel(operation)
    }

    /// Returns lifecycle events retained by the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Retained application events.
    ///
    /// # Errors
    /// This adapter does not return an event access error.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        Ok(self.service.operations().events())
    }
}

impl<S: TuiService> TuiOperationService for S {
    /// Uses the operation method exposed by the TUI service boundary.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Public operation request.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns the service operation error.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        TuiService::execute_operation(self, context, request)
    }
}

/// Executes one public operation through the TUI operation port.
///
/// # Parameters
/// - `service`: TUI operation service.
/// - `context`: Application capability context.
/// - `request`: Operation request to execute.
///
/// # Returns
/// The application operation result.
///
/// # Errors
/// Returns the application error produced by the operation service.
pub fn execute_operation<S: TuiOperationService>(
    service: &S,
    context: &ApplicationContext,
    request: &ApplicationRequest,
) -> Result<ApplicationResult, ApplicationError> {
    service.execute_operation(context, request)
}

/// Executes a non-inspection effect and converts its public result to an action.
pub fn execute_operation_effect<S: TuiOperationService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Action {
    let request_id = match &effect {
        Effect::History { request_id, .. }
        | Effect::Diff { request_id, .. }
        | Effect::Render { request_id, .. }
        | Effect::Apply { request_id, .. }
        | Effect::Commit { request_id, .. } => *request_id,
        _ => return Action::RefreshRequested,
    };
    let request = match effect {
        Effect::History { snapshot, .. } => {
            scriberig_app::ApplicationRequest::History(scriberig_app::HistoryRequest {
                snapshot: scriberig_app::SnapshotSelector::Exact(snapshot),
            })
        }
        Effect::Diff { request, .. } => scriberig_app::ApplicationRequest::Diff(request),
        Effect::Render { request, .. } => scriberig_app::ApplicationRequest::Render(request),
        Effect::Apply { request, .. } => scriberig_app::ApplicationRequest::Apply(request),
        Effect::Commit { request, .. } => scriberig_app::ApplicationRequest::Commit(request),
        _ => unreachable!(),
    };
    match service.execute_operation(context, &request) {
        Ok(scriberig_app::ApplicationResult::History(summary)) => {
            let snapshot = summary.snapshot.unwrap_or_else(|| match &request {
                scriberig_app::ApplicationRequest::History(value) => match value.snapshot {
                    scriberig_app::SnapshotSelector::Exact(snapshot) => snapshot,
                    scriberig_app::SnapshotSelector::NamedRoot(_) => {
                        scriberig_core::SnapshotId::new([0; 32])
                    }
                },
                _ => scriberig_core::SnapshotId::new([0; 32]),
            });
            Action::HistorySucceeded {
                request_id,
                model: crate::view::history::HistoryModel::from_summary(snapshot, &summary),
            }
        }
        Ok(scriberig_app::ApplicationResult::Diff(summary)) => summary.diff.as_ref().map_or_else(
            || Action::OperationFailed {
                request_id,
                category: "invalid_result".to_owned(),
                message: "diff result did not contain semantic changes".to_owned(),
            },
            |diff| Action::DiffSucceeded {
                request_id,
                model: crate::view::diff::DiffModel::from_diff(diff),
            },
        ),
        Ok(scriberig_app::ApplicationResult::Render(summary)) => {
            summary.rendered.as_ref().map_or_else(
                || Action::OperationFailed {
                    request_id,
                    category: "invalid_result".to_owned(),
                    message: "render result did not contain projection content".to_owned(),
                },
                |rendered| Action::ProjectionSucceeded {
                    request_id,
                    model: crate::view::projection::ProjectionModel::from_rendered(rendered),
                },
            )
        }
        Ok(scriberig_app::ApplicationResult::Apply(summary)) => {
            let scriberig_app::ApplicationRequest::Apply(request) = request else {
                unreachable!()
            };
            Action::ReviewSucceeded {
                request_id,
                model: crate::view::changes::ChangeReviewModel::from_summary(
                    request.base_snapshot,
                    request.change,
                    request.branch,
                    &summary,
                ),
            }
        }
        Ok(scriberig_app::ApplicationResult::Commit(summary)) => {
            let scriberig_app::ApplicationRequest::Commit(request) = request else {
                unreachable!()
            };
            Action::ReviewSucceeded {
                request_id,
                model: crate::view::changes::ChangeReviewModel::from_summary(
                    request.base_snapshot,
                    request.change,
                    request.branch,
                    &summary,
                ),
            }
        }
        Ok(_) => Action::OperationFailed {
            request_id,
            category: "invalid_result".to_owned(),
            message: "application returned an unexpected operation result".to_owned(),
        },
        Err(error) => Action::OperationFailed {
            request_id,
            category: error.category().to_owned(),
            message: crate::inspector::sanitize(&error.to_string()),
        },
    }
}

/// Adapts any public application query service to the TUI port.
pub struct ApplicationAdapter<S> {
    service: S,
    capabilities: Vec<RuntimeCapability>,
}

impl<S> ApplicationAdapter<S> {
    /// Creates a TUI adapter around a public application service.
    ///
    /// # Parameters
    /// - `service`: Public query service.
    /// - `capabilities`: Runtime capabilities exposed by the service.
    ///
    /// # Returns
    /// A configured TUI adapter.
    #[must_use]
    pub fn new(service: S, capabilities: Vec<RuntimeCapability>) -> Self {
        Self {
            service,
            capabilities,
        }
    }

    /// Borrows the wrapped application service.
    #[must_use]
    pub const fn service(&self) -> &S {
        &self.service
    }
}

impl<S: QueryService> TuiService for ApplicationAdapter<S> {
    /// Delegates inspection through the public query service.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the query service error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.service.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }

    /// Returns the capabilities supplied by the composition root.
    ///
    /// # Parameters
    /// - `_context`: Application context for capability access.
    ///
    /// # Returns
    /// The configured runtime capabilities.
    ///
    /// # Errors
    /// This adapter does not return a capability error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(self.capabilities.clone())
    }
}

impl<S: QueryService> TuiService for S {
    /// Delegates inspection through a public query service.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the query service error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }
}

/// Executes one semantic effect outside the pure reducer boundary.
#[allow(clippy::needless_pass_by_value)]
pub fn execute_effect<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Action {
    match effect {
        Effect::InspectEntity {
            request_id,
            entity,
            snapshot,
        } => match service.inspect_entity(context, entity, snapshot) {
            Ok(result) => Action::InspectSucceeded {
                request_id,
                model: InspectorModel::from_result(result),
            },
            Err(error) => Action::InspectFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::RequestCapabilities => match service.runtime_capabilities(context) {
            Ok(capabilities) => Action::CapabilitiesLoaded(capabilities),
            Err(error) => Action::InspectFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::History { .. }
        | Effect::Diff { .. }
        | Effect::Render { .. }
        | Effect::Apply { .. }
        | Effect::Commit { .. } => execute_operation_effect(service, context, effect),
        Effect::OperationStatus { operation } => service
            .operation_status(context, &operation)
            .map(|record| {
                Action::OperationObserved(crate::view::operations::OperationModel::from_record(
                    &record,
                ))
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::CancelOperation { operation } => service
            .cancel_operation(context, &operation)
            .map(|record| {
                Action::OperationObserved(crate::view::operations::OperationModel::from_record(
                    &record,
                ))
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::None
        | Effect::RequestRefresh
        | Effect::RequestFind { .. }
        | Effect::RequestSemanticAction { .. }
        | Effect::RequestQuit => Action::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplicationAdapter, TuiService};
    use scriberig_app::{
        ApplicationContext, ApplicationError, InspectRequest, InspectResult, QueryService,
    };
    use scriberig_core::{EntityId, SnapshotId};

    /// Provides a deterministic query result for adapter tests.
    struct Fixture;

    impl QueryService for Fixture {
        /// Returns a deterministic public inspection result.
        ///
        /// # Parameters
        /// - `_context`: Application context for the inspection.
        /// - `request`: Inspection request to resolve.
        ///
        /// # Returns
        /// The deterministic inspection result.
        ///
        /// # Errors
        /// This fixture does not return an application error.
        fn inspect(
            &self,
            _context: &ApplicationContext,
            request: &InspectRequest,
        ) -> Result<InspectResult, ApplicationError> {
            Ok(InspectResult {
                entity: request.entity,
                revision: None,
                snapshot: match request.snapshot {
                    scriberig_app::SnapshotResolution::At(snapshot) => Some(snapshot),
                    _ => None,
                },
                object_type: None,
                label: Some("fixture".to_owned()),
                description: None,
                capabilities: Vec::new(),
                trust: None,
            })
        }
    }

    /// Verifies the adapter preserves exact public snapshot context.
    #[test]
    fn delegates_public_inspection_contract() {
        let adapter = ApplicationAdapter::new(Fixture, Vec::new());
        let entity = EntityId::from_bytes([7; 16]);
        let snapshot = SnapshotId::new([8; 32]);
        let result = adapter
            .inspect_entity(&ApplicationContext::empty(), entity, Some(snapshot))
            .expect("inspection");
        assert_eq!(result.entity, entity);
        assert_eq!(result.snapshot, Some(snapshot));
    }
}
