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
    /// - `_event_kinds`: Argument named `_event_kinds`.
    fn events(&self, _event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "event subscription".to_owned(),
        })
    }

    /// Executes one structured query through the application boundary.
    ///
    /// # Errors
    /// Returns an application error when query execution is unavailable.
    /// - `_context`: Argument named `_context`.
    /// - `_request`: Argument named `_request`.
    ///
    /// # Returns
    /// The value produced by the declaration.
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
    /// - `_context`: Argument named `_context`.
    /// - `_kind`: Argument named `_kind`.
    /// - `_payload`: Argument named `_payload`.
    ///
    /// # Returns
    /// The value produced by the declaration.
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
    /// - `context`: Argument named `context`.
    /// - `request`: Argument named `request`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        (**self).inspect(context, request)
    }

    /// Delegates capability discovery to the borrowed application service.
    ///
    /// # Returns
    /// The value produced by the declaration.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        (**self).capabilities()
    }

    /// Delegates operation status to the borrowed application service.
    /// - `operation_id`: Argument named `operation_id`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        (**self).operation_status(operation_id)
    }

    /// Delegates operation cancellation to the borrowed application service.
    /// - `operation_id`: Argument named `operation_id`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        (**self).cancel_operation(operation_id)
    }

    /// Delegates event observation to the borrowed application service.
    /// - `event_kinds`: Argument named `event_kinds`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn events(&self, event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
        (**self).events(event_kinds)
    }

    /// Delegates structured queries to the borrowed application service.
    /// - `context`: Argument named `context`.
    /// - `request`: Argument named `request`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn query(
        &self,
        context: &ApplicationContext,
        request: &QuerySpec,
    ) -> Result<QueryResult, ApplicationError> {
        (**self).query(context, request)
    }

    /// Delegates declared commands to the borrowed application service.
    /// - `context`: Argument named `context`.
    /// - `kind`: Argument named `kind`.
    /// - `payload`: Argument named `payload`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
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
    /// - `context`: Argument named `context`.
    /// - `request`: Argument named `request`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
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
    /// - `operation_id`: Argument named `operation_id`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        let id = operation_id
            .parse::<OperationId>()
            .map_err(|_| ApplicationError::NotFound {
                subject: operation_id.to_owned(),
            })?;
        Ok(operation_from_record(self.operations().cancel(&id)?))
    }

    /// Exposes buffered lifecycle events as stable protocol envelopes.
    /// - `event_kinds`: Argument named `event_kinds`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
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
    /// - `context`: Argument named `context`.
    /// - `kind`: Argument named `kind`.
    /// - `payload`: Argument named `payload`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
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
/// - `kind`: Argument named `kind`.
/// - `payload`: Argument named `payload`.
///
/// # Returns
/// The value produced by the declaration.
///
/// # Errors
/// Returns an error when the operation cannot complete.
#[path = "server_commands.rs"]
mod server_commands;
#[path = "server_helpers.rs"]
mod server_helpers;
use server_commands::decode_command;
use server_helpers::{
    event_from_application, event_matches, operation_from_record, result_handles,
};
