//! Defines the application service boundary accepted by the server adapter.

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationEvent, ApplicationRequest, ApplyRequest,
    BranchRequest, CommitRequest, DiffRequest, InspectRequest, InspectResult, MigrationDetail,
    OperationId, OperationKind, RenderRequest, RuntimeCapability,
};
use scriberig_protocol::{Event, EventKind, Operation};
use scriberig_query::{FindRequest, FindResult, ListRequest, ListResult, QueryResult, QuerySpec};

/// Application semantics supplied to protocol dispatch.
pub trait ApplicationService {
    /// Reads canonical detail for one migration proposal.
    ///
    /// # Parameters
    /// - `_context`: Trusted request context.
    /// - `_migration_id`: Stable migration handle.
    ///
    /// # Returns
    /// Snapshot-bound migration detail.
    ///
    /// # Errors
    /// Returns unsupported-operation when proposal reads are unavailable.
    fn migration_detail(
        &self,
        _context: &ApplicationContext,
        _migration_id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "migration detail".to_owned(),
        })
    }

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

    /// Reads one compact durable session tree.
    ///
    /// # Parameters
    /// - `_context`: Trusted request context.
    /// - `_session`: Stable session identity.
    ///
    /// # Returns
    /// Provider-neutral session and branch activity summaries.
    ///
    /// # Errors
    /// Returns a structured application error when the session is unavailable.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _session: &str,
    ) -> Result<scriberig_protocol::SessionReadResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session read".to_owned(),
        })
    }

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

    /// Returns public events after a reconnect cursor.
    ///
    /// # Parameters
    /// - `event_kinds`: Event kind filters.
    /// - `after`: Last cursor already processed by the caller.
    ///
    /// # Returns
    /// Retained events newer than `after`.
    ///
    /// # Errors
    /// Returns an error when the replay cursor is no longer retained.
    fn events_after(
        &self,
        event_kinds: &[String],
        after: u64,
    ) -> Result<Vec<Event>, ApplicationError> {
        Ok(self
            .events(event_kinds)?
            .into_iter()
            .filter(|event| event.sequence > after)
            .collect())
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

    /// Executes one bounded semantic collection listing.
    ///
    /// # Parameters
    /// - `_context`: Trusted request context.
    /// - `_request`: Snapshot-bound collection request.
    ///
    /// # Returns
    /// A bounded collection page.
    ///
    /// # Errors
    /// Returns unsupported-operation when listing is unavailable.
    fn list(
        &self,
        _context: &ApplicationContext,
        _request: &ListRequest,
    ) -> Result<ListResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "list".to_owned(),
        })
    }

    /// Executes bounded semantic discovery within an exact scope.
    ///
    /// # Parameters
    /// - `_context`: Trusted request context.
    /// - `_request`: Snapshot-scoped discovery request.
    ///
    /// # Returns
    /// A bounded discovery result.
    ///
    /// # Errors
    /// Returns unsupported-operation when discovery is unavailable.
    fn find(
        &self,
        _context: &ApplicationContext,
        _request: &FindRequest,
    ) -> Result<FindResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "find".to_owned(),
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
    /// Delegates migration detail to the borrowed application service.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `migration_id`: Stable migration handle.
    ///
    /// # Returns
    /// Snapshot-bound migration detail.
    ///
    /// # Errors
    /// Returns the delegated service error.
    fn migration_detail(
        &self,
        context: &ApplicationContext,
        migration_id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        (**self).migration_detail(context, migration_id)
    }

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

    /// Delegates compact session reads to the borrowed service.
    /// - `context`: Trusted request context.
    /// - `session`: Stable session identity.
    ///
    /// # Returns
    /// Compact session tree response.
    ///
    /// # Errors
    /// Returns the service's structured failure.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: &str,
    ) -> Result<scriberig_protocol::SessionReadResponse, ApplicationError> {
        (**self).read_session(context, session)
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

    /// Delegates cursor-based event observation to the borrowed service.
    /// - `event_kinds`: Event kind filters.
    /// - `after`: Last cursor already processed by the caller.
    ///
    /// # Returns
    /// Events newer than `after`.
    ///
    /// # Errors
    /// Returns an error when replay is unavailable.
    fn events_after(
        &self,
        event_kinds: &[String],
        after: u64,
    ) -> Result<Vec<Event>, ApplicationError> {
        (**self).events_after(event_kinds, after)
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

#[path = "server_application.rs"]
mod server_application;

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
pub(crate) mod server_helpers;
