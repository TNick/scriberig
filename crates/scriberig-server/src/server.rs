//! Defines the application service boundary accepted by the server adapter.

#[macro_use]
#[path = "server_presentation_contract.rs"]
mod server_presentation_contract;
#[path = "server_agent_command.rs"]
mod server_agent_command;
#[path = "server_agent_console.rs"]
mod server_agent_console;
#[path = "server_agent_preferences.rs"]
mod server_agent_preferences;

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

    /// Reads one captured safe request by session, branch, and invocation.
    ///
    /// # Parameters
    /// - `_context`: Trusted request context.
    /// - `_session`: Stable session identity.
    /// - `_branch`: Stable branch identity.
    /// - `_invocation`: Durable model invocation identity.
    ///
    /// # Returns
    /// Safe normalized request detail for the selected invocation.
    ///
    /// # Errors
    /// Returns unsupported-operation when detail reads are unavailable.
    fn model_request_detail(
        &self,
        _context: &ApplicationContext,
        _session: &str,
        _branch: &str,
        _invocation: &str,
    ) -> Result<scriberig_protocol::ModelRequestDetailResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model request detail".to_owned(),
        })
    }

    /// Reads expanded bounded detail for one durable tool result.
    ///
    /// # Parameters
    /// - `_context`: Trusted request context.
    /// - `_session`: Stable session identity.
    /// - `_branch`: Stable branch identity.
    /// - `_result`: Durable tool result identity.
    ///
    /// # Returns
    /// Whitelisted safe result detail.
    ///
    /// # Errors
    /// Returns unsupported-operation when detail reads are unavailable.
    fn tool_result_detail(
        &self,
        _context: &ApplicationContext,
        _session: &str,
        _branch: &str,
        _result: &str,
    ) -> Result<scriberig_protocol::ToolResultDetailResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "tool result detail".to_owned(),
        })
    }

    /// Reads safe model descriptors and curated profiles.
    ///
    /// # Parameters
    /// - `_context`: Trusted caller context.
    ///
    /// # Returns
    /// Credential-safe public model library.
    ///
    /// # Errors
    /// Returns unsupported-operation when model configuration is unavailable.
    fn model_library(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model library".to_owned(),
        })
    }

    agent_presentation_methods!();

    /// Changes the model profile used by future session invocations.
    ///
    /// # Parameters
    /// - `_context`: Trusted caller context.
    /// - `_session`: Stable session identity.
    /// - `_expected_revision`: Selection revision observed by the caller.
    /// - `_model_profile`: Curated profile identity to select.
    ///
    /// # Returns
    /// Updated session execution selection.
    ///
    /// # Errors
    /// Returns unsupported-operation when selection mutation is unavailable.
    fn set_session_model(
        &self,
        _context: &ApplicationContext,
        _session: &str,
        _expected_revision: u64,
        _model_profile: &str,
    ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session model selection".to_owned(),
        })
    }

    /// Changes the thinking level used by future session invocations.
    ///
    /// # Parameters
    /// - `_context`: Trusted caller context.
    /// - `_session`: Stable session identity.
    /// - `_expected_revision`: Selection revision observed by the caller.
    /// - `_thinking_level`: Advertised thinking id to select.
    ///
    /// # Returns
    /// Updated session execution selection.
    ///
    /// # Errors
    /// Returns unsupported-operation when selection mutation is unavailable.
    fn set_session_thinking_level(
        &self,
        _context: &ApplicationContext,
        _session: &str,
        _expected_revision: u64,
        _thinking_level: &str,
    ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session thinking selection".to_owned(),
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

#[path = "server_application.rs"]
mod server_application;
#[path = "server_borrowed.rs"]
mod server_borrowed;

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
