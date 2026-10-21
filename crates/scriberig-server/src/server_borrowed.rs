//! Delegates server service calls through borrowed application services.

use super::ApplicationService;
use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, MigrationDetail,
    RuntimeCapability,
};
use scriberig_protocol::{Event, Operation};
use scriberig_query::{FindRequest, FindResult, ListRequest, ListResult, QueryResult, QuerySpec};
impl<S: ApplicationService + ?Sized> ApplicationService for &S {
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

    /// Delegates capability discovery to the borrowed application service.
    ///
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        (**self).capabilities()
    }

    /// Delegates compact session reads to the borrowed service.
    /// - `context`: Trusted request context.
    /// - `session`: Stable session identity.
    ///
    /// Compact session tree response.
    ///
    /// Returns the service's structured failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: &str,
    ) -> Result<scriberig_protocol::SessionReadResponse, ApplicationError> {
        (**self).read_session(context, session)
    }

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

    /// Delegates semantic discovery to the borrowed application service.
    ///
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-scoped discovery request.
    ///
    /// The service's bounded discovery result.
    ///
    /// Returns the service's structured failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn find(
        &self,
        context: &ApplicationContext,
        request: &FindRequest,
    ) -> Result<FindResult, ApplicationError> {
        (**self).find(context, request)
    }

    /// Delegates bounded collection listing to the borrowed service.
    ///
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-bound collection request.
    ///
    /// The service's bounded collection page.
    ///
    /// Returns the service's structured failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn list(
        &self,
        context: &ApplicationContext,
        request: &ListRequest,
    ) -> Result<ListResult, ApplicationError> {
        (**self).list(context, request)
    }

    /// Delegates structured queries to the borrowed application service.
    /// - `context`: Argument named `context`.
    /// - `request`: Argument named `request`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    ///
    /// Returns an error when the operation cannot complete.
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
        (**self).command(context, kind, payload)
    }
}
