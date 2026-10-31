//! Delegates server service calls through borrowed application services.

use super::ApplicationService;
use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, MigrationDetail,
    RuntimeCapability,
};
use scriberig_protocol::{Event, Operation};
use scriberig_query::{FindRequest, FindResult, ListRequest, ListResult, QueryResult, QuerySpec};
#[macro_use]
#[path = "server_borrowed_plan_view.rs"]
mod plan_view_method;
#[macro_use]
#[path = "server_borrowed_recovery.rs"]
mod recovery_methods;
#[macro_use]
#[path = "server_borrowed_artifacts.rs"]
mod artifact_methods;
#[macro_use]
#[path = "server_borrowed_spreadsheet_rename.rs"]
mod spreadsheet_rename_methods;
#[macro_use]
#[path = "server_borrowed_spreadsheet.rs"]
mod spreadsheet_methods;
#[macro_use]
#[path = "server_borrowed_session_read.rs"]
mod session_read_method;
#[macro_use]
#[path = "server_borrowed_projection_style.rs"]
mod projection_style_methods;
#[macro_use]
#[path = "server_borrowed_queries.rs"]
mod query_methods;

impl<S: ApplicationService + ?Sized> ApplicationService for &S {
    server_borrowed_projection_style_methods!();

    /// Delegates project-service composition to the borrowed application.
    ///
    /// # Parameters
    /// - `store`: Durable project catalog and semantic store.
    ///
    /// # Returns
    /// A project service configured by the borrowed host.
    fn project_service(
        &self,
        store: std::sync::Arc<scriberig_store_sqlite::SqliteStore>,
    ) -> scriberig_project::ProjectService {
        (**self).project_service(store)
    }

    /// Delegates migration detail to the borrowed application service.
    ///
    /// - `context`: Trusted request context.
    /// - `migration_id`: Stable migration handle.
    ///
    /// Snapshot-bound migration detail.
    ///
    /// Returns the delegated service error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        (**self).inspect(context, request)
    }

    shared_borrowed_artifact_methods!();
    shared_borrowed_session_read_method!();

    /// Delegates capability discovery to the borrowed application service.
    ///
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        (**self).capabilities()
    }

    /// Delegates CAD provider discovery to the borrowed application service.
    ///
    /// # Returns
    /// The delegated server-side provider state.
    fn cad_provider_capabilities(&self) -> Vec<scriberig_protocol::CadProviderCapabilityInfo> {
        (**self).cad_provider_capabilities()
    }

    shared_borrowed_plan_detail_methods!();
    shared_borrowed_plan_view_method!();

    /// Delegates model request detail reads to the borrowed service.
    ///
    /// - `context`: Trusted request context.
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity.
    /// - `invocation`: Stable model invocation identity.
    /// - `offset`: Optional Unicode-scalar cursor for full detail paging.
    /// - `max_chars`: Optional maximum page character count.
    ///
    /// Safe normalized request detail.
    ///
    /// Returns the service's structured failure.
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
        (**self).model_request_detail(context, session, branch, invocation, offset, max_chars)
    }

    /// Delegates tool result detail reads to the borrowed service.
    ///
    /// - `context`: Trusted request context.
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity.
    /// - `result`: Stable tool result identity.
    ///
    /// Bounded safe result detail.
    ///
    /// Returns the service's structured failure.
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
        (**self).tool_result_detail(context, session, branch, result)
    }

    /// Delegates model-library reads to the borrowed service.
    ///
    /// - `context`: Trusted caller context.
    ///
    /// The service's safe model library.
    ///
    /// Returns the service's structured failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_library(
        &self,
        context: &ApplicationContext,
    ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
        (**self).model_library(context)
    }

    /// Delegates the model-catalog generation read to the borrowed service.
    ///
    /// The generation binds model rows to the credential readiness rows a host
    /// projection resolved from the same upstream catalog state.
    ///
    /// # Returns
    /// The borrowed service's current upstream catalog generation, when it has
    /// an upstream model catalog.
    fn model_catalog_generation(&self) -> Option<scriberig_app::ModelCatalogGeneration> {
        (**self).model_catalog_generation()
    }

    /// Delegates shared presentation reads to the borrowed service.
    ///
    /// - `context`: Trusted caller identity and capabilities.
    ///
    /// Revisioned shared presentation defaults.
    ///
    /// Returns the service's structured failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_presentation_preferences(
        &self,
        context: &ApplicationContext,
    ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
        (**self).agent_presentation_preferences(context)
    }

    /// Delegates shared presentation updates to the borrowed service.
    ///
    /// - `context`: Trusted caller identity and capabilities.
    /// - `request`: Revision-checked complete preference values.
    ///
    /// Updated revisioned preferences.
    ///
    /// Returns the service's structured failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_agent_presentation_preferences(
        &self,
        context: &ApplicationContext,
        request: &scriberig_protocol::AgentPresentationPreferencesSetRequest,
    ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
        (**self).set_agent_presentation_preferences(context, request)
    }

    /// Delegates upstream provider/model selection to the borrowed service.
    ///
    /// - `context`: Trusted caller context.
    /// - `session`: Stable session identity.
    /// - `expected_revision`: Selection revision observed by the caller.
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-scoped upstream model identity.
    ///
    /// The updated session execution selection.
    ///
    /// Returns the service's structured failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_model(
        &self,
        context: &ApplicationContext,
        session: &str,
        expected_revision: u64,
        provider_id: &str,
        model_id: &str,
    ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
        (**self).set_session_model(context, session, expected_revision, provider_id, model_id)
    }

    /// Delegates thinking-level selection to the borrowed service.
    ///
    /// - `context`: Trusted caller context.
    /// - `session`: Stable session identity.
    /// - `expected_revision`: Selection revision observed by the caller.
    /// - `thinking_level`: Advertised thinking capability id.
    ///
    /// The updated session execution selection.
    ///
    /// Returns the service's structured failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_thinking_level(
        &self,
        context: &ApplicationContext,
        session: &str,
        expected_revision: u64,
        thinking_level: &str,
    ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
        (**self).set_session_thinking_level(context, session, expected_revision, thinking_level)
    }

    server_borrowed_recovery_methods!();

    /// Delegates operation status to the borrowed application service.
    /// - `operation_id`: Argument named `operation_id`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        (**self).operation_status(operation_id)
    }

    /// Delegates operation cancellation to the borrowed application service.
    /// - `operation_id`: Argument named `operation_id`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        (**self).cancel_operation(operation_id)
    }

    /// Delegates event observation to the borrowed application service.
    /// - `event_kinds`: Argument named `event_kinds`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn events(&self, event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
        (**self).events(event_kinds)
    }

    /// Delegates cursor-based event observation to the borrowed service.
    /// - `event_kinds`: Event kind filters.
    /// - `after`: Last cursor already processed by the caller.
    ///
    /// Events newer than `after`.
    ///
    /// Returns an error when replay is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn events_after(
        &self,
        event_kinds: &[String],
        after: u64,
    ) -> Result<Vec<Event>, ApplicationError> {
        (**self).events_after(event_kinds, after)
    }

    server_borrowed_query_methods!();
}
