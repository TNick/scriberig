//! Defines the application service boundary accepted by the server adapter.

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationEvent, ApplicationRequest, ApplyRequest,
    BranchRequest, CommitRequest, DiffRequest, InspectRequest, InspectResult, OperationId,
    OperationKind, RuntimeCapability,
};
use scriberig_protocol::{Event, EventKind, Operation};
use scriberig_query::{QueryResult, QuerySpec};

/// Application semantics supplied to protocol dispatch.
pub trait ApplicationService {
    /// Executes one application inspection.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// # Returns
    /// Application inspection result.
    ///
    /// # Errors
    /// Returns a structured application error.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError>;

    /// Returns currently available runtime capabilities.
    ///
    /// # Returns
    /// Runtime capability descriptors.
    fn capabilities(&self) -> Vec<RuntimeCapability>;

    /// Returns one operation status in protocol-independent form.
    ///
    /// # Parameters
    /// - `operation_id`: Stable operation identity.
    ///
    /// # Returns
    /// Operation status ready for protocol mapping.
    ///
    /// # Errors
    /// Returns a structured application error when status is unavailable.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError>;

    /// Cancels one tracked operation when the application supports cancellation.
    ///
    /// # Parameters
    /// - `operation_id`: Stable operation identity.
    ///
    /// # Returns
    /// The cancelled operation status.
    ///
    /// # Errors
    /// Returns unsupported-operation when cancellation is unavailable.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: format!("operation cancellation {operation_id}"),
        })
    }

    /// Returns currently buffered public events matching the requested kinds.
    ///
    /// # Parameters
    /// - `event_kinds`: Event kind filters; empty selects all available events.
    ///
    /// # Returns
    /// Stable event envelopes.
    ///
    /// # Errors
    /// Returns unsupported-operation when event delivery is unavailable.
    fn events(&self, _event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "event subscription".to_owned(),
        })
    }

    /// Executes one structured query through the application boundary.
    ///
    /// # Errors
    /// Returns an application error when query execution is unavailable.
    fn query(
        &self,
        _context: &ApplicationContext,
        _request: &QuerySpec,
    ) -> Result<QueryResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "query".to_owned(),
        })
    }

    /// Executes one declared application command and returns its lifecycle state.
    ///
    /// # Errors
    /// Returns an application error when command execution is unavailable.
    fn command(
        &self,
        _context: &ApplicationContext,
        _kind: &str,
        _payload: &serde_json::Value,
    ) -> Result<Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "command".to_owned(),
        })
    }
}

impl<S: ApplicationService + ?Sized> ApplicationService for &S {
    /// Delegates inspection to the borrowed application service.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        (**self).inspect(context, request)
    }

    /// Delegates capability discovery to the borrowed application service.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        (**self).capabilities()
    }

    /// Delegates operation status to the borrowed application service.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        (**self).operation_status(operation_id)
    }

    /// Delegates operation cancellation to the borrowed application service.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        (**self).cancel_operation(operation_id)
    }

    /// Delegates event observation to the borrowed application service.
    fn events(&self, event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
        (**self).events(event_kinds)
    }

    /// Delegates structured queries to the borrowed application service.
    fn query(
        &self,
        context: &ApplicationContext,
        request: &QuerySpec,
    ) -> Result<QueryResult, ApplicationError> {
        (**self).query(context, request)
    }

    /// Delegates declared commands to the borrowed application service.
    fn command(
        &self,
        context: &ApplicationContext,
        kind: &str,
        payload: &serde_json::Value,
    ) -> Result<Operation, ApplicationError> {
        (**self).command(context, kind, payload)
    }
}

// Adapt the shared application façade to the protocol server boundary.
impl<B> ApplicationService for scriberig_app::ApplicationService<B>
where
    B: scriberig_app::ApplicationBackend,
{
    /// Executes inspection through the shared application façade.
    ///
    /// # Parameters
    /// - `self`: Shared application service.
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// # Returns
    /// Application inspection result.
    ///
    /// # Errors
    /// Returns the shared application failure.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        scriberig_app::QueryService::inspect(self, context, request)
    }

    /// Advertises the generic inspect capability supplied by this façade.
    ///
    /// # Returns
    /// One available inspect capability.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        vec![RuntimeCapability::new(
            scriberig_app::CapabilityId::new("inspect_entity").expect("static capability"),
            "Inspect one entity",
            true,
        )]
    }

    /// Executes a structured query through the shared application backend.
    fn query(
        &self,
        context: &ApplicationContext,
        request: &QuerySpec,
    ) -> Result<QueryResult, ApplicationError> {
        match self.execute(
            context,
            &scriberig_app::ApplicationRequest::Query(request.clone()),
        )? {
            scriberig_app::ApplicationResult::Query(result) => Ok(result),
            _ => Err(ApplicationError::Internal {
                message: "query backend returned a different result kind".to_owned(),
            }),
        }
    }

    /// Reads operation state from the shared application lifecycle registry.
    ///
    /// # Parameters
    /// - `self`: Shared application service.
    /// - `operation_id`: Requested operation identity.
    ///
    /// # Returns
    /// # Errors
    /// Returns a structured error when the operation is unknown.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        let id = operation_id
            .parse::<OperationId>()
            .map_err(|_| ApplicationError::NotFound {
                subject: operation_id.to_owned(),
            })?;
        Ok(operation_from_record(self.operations().get(&id)?))
    }

    /// Cancels operation state through the shared application lifecycle registry.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        let id = operation_id
            .parse::<OperationId>()
            .map_err(|_| ApplicationError::NotFound {
                subject: operation_id.to_owned(),
            })?;
        Ok(operation_from_record(self.operations().cancel(&id)?))
    }

    /// Exposes buffered lifecycle events as stable protocol envelopes.
    fn events(&self, event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
        Ok(self
            .operations()
            .events()
            .into_iter()
            .map(event_from_application)
            .filter(|event| event_matches(event, event_kinds))
            .collect())
    }

    /// Decodes and executes one command through the shared application façade.
    fn command(
        &self,
        context: &ApplicationContext,
        kind: &str,
        payload: &serde_json::Value,
    ) -> Result<Operation, ApplicationError> {
        let (operation_kind, request) = decode_command(kind, payload)?;
        let operation_id = OperationId::new(format!(
            "command-{kind}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| ApplicationError::Internal {
                    message: error.to_string(),
                })?
                .as_nanos()
        ))
        .map_err(|error| ApplicationError::Internal {
            message: error.to_string(),
        })?;
        self.operations()
            .start(operation_id.clone(), operation_kind)?;
        self.operations()
            .transition(&operation_id, scriberig_app::OperationStatus::Running)?;
        let result = self.execute(context, &request);
        match result {
            Ok(result) => {
                let handles = result_handles(&result);
                Ok(operation_from_record(
                    self.operations().complete(&operation_id, handles)?,
                ))
            }
            Err(error) => {
                let _ = self
                    .operations()
                    .transition(&operation_id, scriberig_app::OperationStatus::Failed);
                Err(error)
            }
        }
    }
}

/// Decodes a protocol command into the transport-independent application model.
fn decode_command(
    kind: &str,
    payload: &serde_json::Value,
) -> Result<(OperationKind, ApplicationRequest), ApplicationError> {
    fn invalid(kind: &str, error: impl std::fmt::Display) -> ApplicationError {
        ApplicationError::InvalidRequest {
            operation: kind.to_owned(),
            message: error.to_string(),
        }
    }

    match kind {
        "apply" => serde_json::from_value::<ApplyRequest>(payload.clone())
            .map(|request| (OperationKind::Apply, ApplicationRequest::Apply(request)))
            .map_err(|error| invalid(kind, error)),
        "branch" => serde_json::from_value::<BranchRequest>(payload.clone())
            .map(|request| (OperationKind::Branch, ApplicationRequest::Branch(request)))
            .map_err(|error| invalid(kind, error)),
        "commit" => serde_json::from_value::<CommitRequest>(payload.clone())
            .map(|request| (OperationKind::Commit, ApplicationRequest::Commit(request)))
            .map_err(|error| invalid(kind, error)),
        "diff" => serde_json::from_value::<DiffRequest>(payload.clone())
            .map(|request| (OperationKind::Diff, ApplicationRequest::Diff(request)))
            .map_err(|error| invalid(kind, error)),
        _ => Err(ApplicationError::UnsupportedOperation {
            operation: format!("command {kind}"),
        }),
    }
}

/// Extracts stable result handles without exposing canonical result payloads.
fn result_handles(result: &scriberig_app::ApplicationResult) -> Vec<String> {
    let summary = match result {
        scriberig_app::ApplicationResult::Diff(value)
        | scriberig_app::ApplicationResult::History(value)
        | scriberig_app::ApplicationResult::Render(value)
        | scriberig_app::ApplicationResult::Branch(value)
        | scriberig_app::ApplicationResult::Apply(value)
        | scriberig_app::ApplicationResult::Commit(value) => Some(value),
        _ => None,
    };
    summary.map_or_else(Vec::new, |value| vec![value.handle.clone()])
}

/// Tests whether a protocol event matches a case-insensitive kind filter.
///
/// # Parameters
/// - `event`: Event whose kind is being tested.
/// - `event_kinds`: Requested event kind names; empty selects every event.
///
/// # Returns
/// `true` when the event is selected by the subscription.
fn event_matches(event: &Event, event_kinds: &[String]) -> bool {
    event_kinds.is_empty()
        || event_kinds.iter().any(|kind| {
            kind.eq_ignore_ascii_case(&format!("{:?}", event.kind))
                || kind.eq_ignore_ascii_case(&format!("{:?}", event.kind).replace('_', ""))
        })
}

/// Converts an application operation record to its public lifecycle form.
fn operation_from_record(record: scriberig_app::OperationRecord) -> Operation {
    Operation {
        reference: scriberig_protocol::OperationReference {
            id: record.reference.to_string(),
        },
        kind: match record.kind {
            scriberig_app::OperationKind::Inspect => {
                scriberig_protocol::operation::OperationKind::Inspect
            }
            scriberig_app::OperationKind::Query => {
                scriberig_protocol::operation::OperationKind::Query
            }
            scriberig_app::OperationKind::Apply => {
                scriberig_protocol::operation::OperationKind::Apply
            }
            scriberig_app::OperationKind::List => {
                scriberig_protocol::operation::OperationKind::List
            }
            scriberig_app::OperationKind::Follow => {
                scriberig_protocol::operation::OperationKind::Follow
            }
            scriberig_app::OperationKind::Find => {
                scriberig_protocol::operation::OperationKind::Find
            }
            scriberig_app::OperationKind::Diff => {
                scriberig_protocol::operation::OperationKind::Diff
            }
            scriberig_app::OperationKind::History => {
                scriberig_protocol::operation::OperationKind::History
            }
            scriberig_app::OperationKind::Render => {
                scriberig_protocol::operation::OperationKind::Render
            }
            scriberig_app::OperationKind::Branch => {
                scriberig_protocol::operation::OperationKind::Branch
            }
            scriberig_app::OperationKind::Commit => {
                scriberig_protocol::operation::OperationKind::Commit
            }
        },
        status: match record.status {
            scriberig_app::OperationStatus::Queued => {
                scriberig_protocol::operation::OperationStatus::Queued
            }
            scriberig_app::OperationStatus::Running => {
                scriberig_protocol::operation::OperationStatus::Running
            }
            scriberig_app::OperationStatus::Completed => {
                scriberig_protocol::operation::OperationStatus::Completed
            }
            scriberig_app::OperationStatus::Failed => {
                scriberig_protocol::operation::OperationStatus::Failed
            }
            scriberig_app::OperationStatus::Cancelled => {
                scriberig_protocol::operation::OperationStatus::Cancelled
            }
        },
        progress: record.progress.map(|progress| {
            let percent = progress
                .total
                .filter(|total| *total > 0)
                .map_or(0, |total| {
                    progress
                        .completed
                        .unwrap_or_default()
                        .saturating_mul(100)
                        .checked_div(total)
                        .unwrap_or(0)
                        .min(100) as u8
                });
            scriberig_protocol::OperationProgress {
                percent,
                message: progress.phase,
            }
        }),
        result_snapshot: None,
        result_handles: record.result_handles,
        error: record.error.map(|error| error.to_string()),
    }
}

/// Converts one application lifecycle event to a stable protocol envelope.
fn event_from_application(event: ApplicationEvent) -> Event {
    let mut entity = None;
    let mut revision = None;
    let mut snapshot = None;
    let mut operation = None;
    let mut metadata = Vec::new();
    let kind = match event {
        ApplicationEvent::OperationStarted { operation: value } => {
            operation = Some(value);
            EventKind::OperationStarted
        }
        ApplicationEvent::OperationProgressed {
            operation: value, ..
        } => {
            operation = Some(value);
            EventKind::OperationProgressed
        }
        ApplicationEvent::OperationCompleted { operation: value } => {
            operation = Some(value);
            EventKind::OperationCompleted
        }
        ApplicationEvent::SnapshotChanged { snapshot: value } => {
            snapshot = Some(scriberig_protocol::SnapshotReference {
                id: value.to_string(),
            });
            EventKind::SnapshotChanged
        }
        ApplicationEvent::EntityChanged {
            entity: value,
            revision: value_revision,
        } => {
            entity = Some(scriberig_protocol::EntityReference {
                id: value.to_string(),
            });
            revision = Some(scriberig_protocol::RevisionReference {
                entity: scriberig_protocol::EntityReference {
                    id: value.to_string(),
                },
                id: value_revision.to_string(),
            });
            EventKind::EntityChanged
        }
        ApplicationEvent::ProjectionChanged { subject } => {
            entity = Some(scriberig_protocol::EntityReference {
                id: subject.to_string(),
            });
            EventKind::ProjectionChanged
        }
        ApplicationEvent::BranchChanged { branch } => {
            metadata.push(("branch".to_owned(), branch));
            EventKind::BranchChanged
        }
        ApplicationEvent::MigrationCreated { migration } => {
            metadata.push(("migration".to_owned(), migration));
            EventKind::MigrationCreated
        }
        ApplicationEvent::ValidationCompleted { subject, passed } => {
            metadata.push(("subject".to_owned(), subject));
            metadata.push(("passed".to_owned(), passed.to_string()));
            EventKind::ValidationCompleted
        }
        ApplicationEvent::AgentOutputProduced { output } => {
            metadata.push(("output".to_owned(), output));
            EventKind::AgentOutputProduced
        }
    };
    Event {
        kind,
        entity,
        revision,
        snapshot,
        operation: operation.map(|value| scriberig_protocol::OperationReference {
            id: value.to_string(),
        }),
        metadata,
    }
}
