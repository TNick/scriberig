//! Defines the application service boundary accepted by the server adapter.

#[macro_use]
#[path = "server_query_contract.rs"]
mod server_query_contract;
#[macro_use]
#[path = "server_presentation_contract.rs"]
mod server_presentation_contract;
#[path = "server_agent_command.rs"]
pub(crate) mod server_agent_command;
#[path = "server_agent_console.rs"]
mod server_agent_console;
#[path = "server_agent_preferences.rs"]
mod server_agent_preferences;
#[macro_use]
#[path = "server_session_operations.rs"]
pub(crate) mod server_session_operations;
#[macro_use]
#[path = "server_plan_operations.rs"]
mod server_plan_operations;
#[path = "server_spreadsheet.rs"]
mod server_spreadsheet;

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationEvent, ApplicationRequest, ApplyRequest,
    BranchRequest, CommitRequest, DiffRequest, InspectRequest, InspectResult, MigrationDetail,
    OperationId, OperationKind, RenderRequest, RuntimeCapability,
};
use scriberig_protocol::{Event, EventKind, Operation};
use scriberig_query::{FindRequest, FindResult, ListRequest, ListResult, QueryResult, QuerySpec};

#[macro_use]
#[path = "server_recovery_contract.rs"]
mod recovery_contract;
#[macro_use]
#[path = "server_document_contract.rs"]
mod document_contract;
#[macro_use]
#[path = "server_cad_contract.rs"]
mod cad_contract;
#[macro_use]
#[path = "server_rich_artifact_contract.rs"]
mod rich_artifact_contract;
#[macro_use]
#[path = "server_spreadsheet_rename_contract.rs"]
mod spreadsheet_rename_contract;
#[macro_use]
#[path = "server_projection_style_contract.rs"]
mod projection_style_contract;

#[macro_use]
#[path = "server_events_contract.rs"]
mod server_events_contract;

/// Application semantics supplied to protocol dispatch.
pub trait ApplicationService {
    document_edit_contract!();
    shared_rich_artifact_contract!();
    shared_cad_contract!();
    spreadsheet_rename_contract!();
    server_projection_style_contract!();

    /// Creates a project service configured with this host's injected providers.
    ///
    /// Product routes use this factory instead of constructing project services
    /// directly, preserving one provider configuration for scans, imports, and
    /// artifact fidelity.
    ///
    /// # Parameters
    /// - `store`: Durable project catalog and semantic store.
    ///
    /// # Returns
    /// A project service for the current server application.
    fn project_service(
        &self,
        store: std::sync::Arc<scriberig_store_sqlite::SqliteStore>,
    ) -> scriberig_project::ProjectService {
        scriberig_project::ProjectService::new(store)
    }

    /// Reads canonical detail for one migration proposal.
    ///
    /// - `_context`: Trusted request context.
    /// - `_migration_id`: Stable migration handle.
    ///
    /// Snapshot-bound migration detail.
    ///
    /// Returns unsupported-operation when proposal reads are unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// Application inspection result.
    ///
    /// Returns a structured application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError>;

    /// Returns currently available runtime capabilities.
    ///
    /// Runtime capability descriptors.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<RuntimeCapability>;

    /// Reports provider-neutral CAD format capabilities available on this server.
    ///
    /// Hosts with injected providers should override this method with their actual
    /// initialized provider descriptors. The default reports DWG as unavailable.
    ///
    /// # Returns
    /// Server-side CAD provider state; this is independent of client machine libraries.
    fn cad_provider_capabilities(&self) -> Vec<scriberig_protocol::CadProviderCapabilityInfo> {
        vec![scriberig_protocol::CadProviderCapabilityInfo {
            format: scriberig_protocol::CadProviderFormat::Dwg,
            availability: scriberig_protocol::CadProviderAvailability::Unavailable,
            provider_id: None,
            provider_version: None,
            can_read: false,
            can_write: false,
            can_validate: false,
            opaque_preservation: scriberig_protocol::CadOpaquePreservation::Unproven,
        }]
    }

    /// Reads one compact durable session tree.
    ///
    /// - `_context`: Trusted request context.
    /// - `_session`: Stable session identity.
    ///
    /// Provider-neutral session and branch activity summaries.
    ///
    /// Returns a structured application error when the session is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _session: &str,
    ) -> Result<scriberig_protocol::SessionReadResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session read".to_owned(),
        })
    }

    shared_plan_attempt_read_contract!();
    shared_plan_details_read_contract!();
    shared_plan_view_read_contract!();

    /// Starts a session through the shared application service.
    ///
    /// - `_context`: Trusted operation context.
    /// - `_request`: Provider-neutral start request.
    ///
    /// Authoritative session, root branch, and operation handles.
    ///
    /// Returns an application authorization or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_new_session(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_protocol::AgentNewSessionRequest,
    ) -> Result<scriberig_protocol::AgentAcceptedResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "agent new session".to_owned(),
        })
    }

    /// Continues at one exact shared session anchor.
    ///
    /// - `_context`: Trusted operation context.
    /// - `_request`: Provider-neutral exact continuation request.
    ///
    /// Authoritative session, child branch, and operation handles.
    ///
    /// Returns an application authorization, anchor, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_continue_at(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_protocol::AgentContinueAtRequest,
    ) -> Result<scriberig_protocol::AgentAcceptedResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "agent continue at anchor".to_owned(),
        })
    }

    /// Applies one typed plan request through shared application operations.
    ///
    /// - `_context`: Trusted caller capability context.
    /// - `_request`: Typed plan creation or mutation request.
    ///
    /// The authoritative plan projection after the operation.
    ///
    /// Returns authorization, validation, conflict, or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn plan_operation(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_protocol::Request,
    ) -> Result<scriberig_protocol::SessionPlan, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "plan operation".to_owned(),
        })
    }

    /// Reads one captured safe request by session, branch, and invocation.
    ///
    /// - `_context`: Trusted request context.
    /// - `_session`: Stable session identity.
    /// - `_branch`: Stable branch identity.
    /// - `_invocation`: Durable model invocation identity.
    /// - `_offset`: Optional Unicode-scalar cursor for full detail paging.
    /// - `_max_chars`: Optional maximum page character count.
    ///
    /// Safe normalized request detail for the selected invocation.
    ///
    /// Returns unsupported-operation when detail reads are unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_request_detail(
        &self,
        _context: &ApplicationContext,
        _session: &str,
        _branch: &str,
        _invocation: &str,
        _offset: Option<u64>,
        _max_chars: Option<u32>,
    ) -> Result<scriberig_protocol::ModelRequestDetailResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model request detail".to_owned(),
        })
    }

    /// Reads expanded bounded detail for one durable tool result.
    ///
    /// - `_context`: Trusted request context.
    /// - `_session`: Stable session identity.
    /// - `_branch`: Stable branch identity.
    /// - `_result`: Durable tool result identity.
    ///
    /// Whitelisted safe result detail.
    ///
    /// Returns unsupported-operation when detail reads are unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `_context`: Trusted caller context.
    ///
    /// Credential-safe public model library.
    ///
    /// Returns unsupported-operation when model configuration is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_library(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model library".to_owned(),
        })
    }

    agent_presentation_methods!();

    server_recovery_contract_methods!();

    /// Returns one operation status in protocol-independent form.
    ///
    /// - `operation_id`: Stable operation identity.
    ///
    /// Operation status ready for protocol mapping.
    ///
    /// Returns a structured application error when status is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError>;

    /// Cancels one tracked operation when the application supports cancellation.
    ///
    /// - `operation_id`: Stable operation identity.
    ///
    /// The cancelled operation status.
    ///
    /// Returns unsupported-operation when cancellation is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: format!("operation cancellation {operation_id}"),
        })
    }

    /// Returns currently buffered public events matching the requested kinds.
    ///
    /// - `event_kinds`: Event kind filters; empty selects all available events.
    ///
    /// Stable event envelopes.
    ///
    /// Returns unsupported-operation when event delivery is unavailable.
    /// - `_event_kinds`: Argument named `_event_kinds`.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn events(&self, _event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "event subscription".to_owned(),
        })
    }

    server_events_contract!();

    server_query_contract!();
}

#[path = "server_application.rs"]
mod server_application;
#[path = "server_borrowed.rs"]
mod server_borrowed;
#[path = "server_plan_attempts.rs"]
mod server_plan_attempts;
#[path = "server_plan_details.rs"]
mod server_plan_details;
#[path = "server_plan_view.rs"]
mod server_plan_view;

/// Decodes a protocol command into the transport-independent application model.
/// - `kind`: Argument named `kind`.
/// - `payload`: Argument named `payload`.
///
///
/// Returns an error when the operation cannot complete.
#[path = "server_commands.rs"]
mod server_commands;
#[path = "server_helpers.rs"]
pub(crate) mod server_helpers;
