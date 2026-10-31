//! Adapts the shared application façade to the protocol server boundary.
#[macro_use]
#[path = "server_application_selection.rs"]
mod selection_methods;

#[macro_use]
#[path = "server_application_presentation.rs"]
mod presentation_methods;
#[macro_use]
#[path = "server_application_plan_view.rs"]
mod plan_view_method;
#[macro_use]
#[path = "server_application_event_methods.rs"]
mod event_methods;
use super::server_agent_console;
use super::server_helpers::{
    event_from_application, event_matches, operation_from_record, session_response_compact,
};
use super::{
    ApplicationContext, ApplicationError, ApplicationService, InspectRequest, InspectResult,
    MigrationDetail, OperationId, RuntimeCapability,
};
#[path = "server_capabilities.rs"]
mod server_capabilities;
use scriberig_protocol::{Event, Operation};
use scriberig_query::{FindRequest, FindResult, ListRequest, ListResult, QueryResult, QuerySpec};
use server_capabilities::{cad_provider_capability, capabilities};
#[path = "server_event_cursor.rs"]
mod server_event_cursor;
use server_event_cursor::filter_events_after;

#[macro_use]
#[path = "server_application_spreadsheet.rs"]
mod spreadsheet_methods;
#[macro_use]
#[path = "server_application_document.rs"]
mod document_methods;
#[macro_use]
#[path = "server_application_geo.rs"]
mod geo_methods;
#[macro_use]
#[path = "server_application_projection_style.rs"]
mod projection_style_methods;
#[macro_use]
#[path = "server_application_rich_artifact_methods.rs"]
mod rich_artifact_methods;

#[macro_use]
#[path = "server_application_query.rs"]
mod query_methods;

// Adapt the shared application façade to the protocol server boundary.
impl<B> ApplicationService for scriberig_app::ApplicationService<B>
where
    B: scriberig_app::ApplicationBackend,
{
    server_application_projection_style_methods!();
    server_application_rich_artifact_methods!();

    /// Creates a project service using providers configured on the application.
    ///
    /// # Parameters
    /// - `store`: Durable project catalog and semantic store.
    ///
    /// # Returns
    /// A project service carrying the exact host-injected provider registry.
    fn project_service(
        &self,
        store: std::sync::Arc<scriberig_store_sqlite::SqliteStore>,
    ) -> scriberig_project::ProjectService {
        self.configured_project_service()
            .unwrap_or_else(|| scriberig_project::ProjectService::new(store))
    }

    /// Reads migration detail through the shared application façade.
    ///
    /// - `self`: Shared application service.
    /// - `context`: Trusted request context.
    /// - `migration_id`: Stable migration handle.
    ///
    /// Snapshot-bound migration detail.
    ///
    /// Returns the shared application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn migration_detail(
        &self,
        context: &ApplicationContext,
        migration_id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        self.migration_detail(context, migration_id)
    }

    /// Executes inspection through the shared application façade.
    ///
    /// - `self`: Shared application service.
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// Application inspection result.
    ///
    /// Returns the shared application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        scriberig_app::QueryService::inspect(self, context, request)
    }

    spreadsheet_methods!();
    document_methods!();
    geo_methods!();

    /// Reads a compact session tree through the shared session boundary.
    ///
    /// - `context`: Trusted request context.
    /// - `session`: Serialized stable session identity.
    ///
    /// Provider-neutral session activity response.
    ///
    /// Returns a schema or application error when the session cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: &str,
    ) -> Result<scriberig_protocol::SessionReadResponse, ApplicationError> {
        let session_id = session
            .parse::<scriberig_core::EntityId>()
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "session read".to_owned(),
                message: format!("invalid session identifier: {error}"),
            })?;
        let value = scriberig_app::SessionBackend::read_session_projection(
            self,
            context,
            &scriberig_app::SessionRequest {
                session: session_id,
            },
        )?;
        session_response_compact(value, |session, plan| {
            self.read_plan_views(context, session, plan, None)
        })
    }

    /// Reads a bounded attempt page from the authorized semantic session.
    ///
    /// # Parameters
    /// - `context`: Trusted session-read capability context.
    /// - `request`: Session, plan, node, and page selection.
    ///
    /// # Returns
    /// Safe attempt metadata without branch transcript content.
    ///
    /// # Errors
    /// Returns authorization, lookup, or invalid-identity errors.
    fn read_plan_attempts(
        &self,
        context: &ApplicationContext,
        request: &scriberig_protocol::SessionPlanAttemptsRequest,
    ) -> Result<scriberig_protocol::SessionPlanAttemptsResponse, ApplicationError> {
        super::server_plan_attempts::read(self, context, request)
    }

    /// Reads a bounded gate and evidence page through shared session semantics.
    ///
    /// # Parameters
    /// - `context`: Trusted session-read capability context.
    /// - `request`: Session, plan revision, and independent page offsets.
    ///
    /// # Returns
    /// Typed requirement records bound to the requested plan revision.
    ///
    /// # Errors
    /// Returns authorization, lookup, stale revision, or identity errors.
    fn read_plan_details(
        &self,
        context: &ApplicationContext,
        request: &scriberig_protocol::SessionPlanDetailsRequest,
    ) -> Result<scriberig_protocol::SessionPlanDetailsResponse, ApplicationError> {
        super::server_plan_details::read(self, context, request)
    }

    shared_plan_view_method!();

    /// Reads a previously captured model request from an authorized session.
    ///
    /// - `context`: Trusted request context.
    /// - `session`: Serialized session identity.
    /// - `branch`: Serialized branch identity.
    /// - `invocation`: Serialized model invocation identity.
    /// - `offset`: Optional Unicode-scalar cursor for full detail paging.
    /// - `max_chars`: Optional maximum page character count.
    ///
    /// Bounded safe request detail captured before provider serialization.
    ///
    /// Returns invalid-request or not-found errors for unavailable records.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_request_detail(
        &self,
        context: &ApplicationContext,
        session: &str,
        branch: &str,
        invocation: &str,
        offset: Option<u64>,
        max_chars: Option<u32>,
    ) -> Result<scriberig_protocol::ModelRequestDetailResponse, ApplicationError> {
        server_agent_console::model_request_detail(
            self, context, session, branch, invocation, offset, max_chars,
        )
    }

    /// Reads bounded safe tool-result detail from an authorized session.
    ///
    /// - `context`: Trusted request context.
    /// - `session`: Serialized session identity.
    /// - `branch`: Serialized branch identity.
    /// - `result`: Serialized tool result identity.
    ///
    /// Safe result detail captured by the trusted tool boundary.
    ///
    /// Returns invalid-request or not-found for unavailable records.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn tool_result_detail(
        &self,
        context: &ApplicationContext,
        session: &str,
        branch: &str,
        result: &str,
    ) -> Result<scriberig_protocol::ToolResultDetailResponse, ApplicationError> {
        server_agent_console::tool_result_detail(self, context, session, branch, result)
    }

    agent_presentation_impl_methods!();
    shared_session_operation_methods!();
    shared_plan_operation_methods!();

    server_execution_selection_methods!();

    /// Advertises the generic inspect capability supplied by this façade.
    ///
    /// One available inspect capability.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        capabilities()
    }

    /// Reports configured CAD providers from the shared application registry.
    ///
    /// # Returns
    /// Provider-neutral status for each configured format and explicit DWG absence.
    fn cad_provider_capabilities(&self) -> Vec<scriberig_protocol::CadProviderCapabilityInfo> {
        self.cad_provider_statuses()
            .into_iter()
            .map(cad_provider_capability)
            .collect()
    }

    server_application_query_methods!();

    /// Reads operation state from the shared application lifecycle registry.
    ///
    /// - `self`: Shared application service.
    /// - `operation_id`: Requested operation identity.
    ///
    /// Returns a structured error when the operation is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        let id = operation_id
            .parse::<OperationId>()
            .map_err(|_| ApplicationError::NotFound {
                subject: operation_id.to_owned(),
            })?;
        Ok(operation_from_record(self.operations().cancel(&id)?))
    }

    server_application_event_methods!();

    /// Decodes and executes one command through the shared application boundary.
    ///
    /// - `context`: Trusted request context.
    /// - `kind`: Stable command family.
    /// - `payload`: Typed command payload.
    ///
    /// The accepted or completed shared operation.
    ///
    /// Returns a structured application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn command(
        &self,
        context: &ApplicationContext,
        kind: &str,
        payload: &serde_json::Value,
    ) -> Result<Operation, ApplicationError> {
        super::server_agent_command::execute(self, context, kind, payload)
    }
}
