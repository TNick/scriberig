//! Adapts the shared application façade to the protocol server boundary.

#[macro_use]
#[path = "server_application_presentation.rs"]
mod presentation_methods;
use super::server_agent_console;
use super::server_helpers::{
    event_from_application, event_matches, operation_from_record, session_response,
};
use super::{
    ApplicationContext, ApplicationError, ApplicationService, InspectRequest, InspectResult,
    MigrationDetail, OperationId, RuntimeCapability,
};
#[path = "server_capabilities.rs"]
mod server_capabilities;
use scriberig_protocol::{Event, Operation};
use scriberig_query::{FindRequest, FindResult, ListRequest, ListResult, QueryResult, QuerySpec};
use server_capabilities::capabilities;
#[path = "server_event_cursor.rs"]
mod server_event_cursor;
use server_event_cursor::filter_events_after;

// Adapt the shared application façade to the protocol server boundary.
impl<B> ApplicationService for scriberig_app::ApplicationService<B>
where
    B: scriberig_app::ApplicationBackend,
{
    /// Reads migration detail through the shared application façade.
    ///
    /// # Parameters
    /// - `self`: Shared application service.
    /// - `context`: Trusted request context.
    /// - `migration_id`: Stable migration handle.
    ///
    /// # Returns
    /// Snapshot-bound migration detail.
    ///
    /// # Errors
    /// Returns the shared application failure.
    fn migration_detail(
        &self,
        context: &ApplicationContext,
        migration_id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        self.migration_detail(context, migration_id)
    }

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

    /// Reads a compact session tree through the shared session boundary.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `session`: Serialized stable session identity.
    ///
    /// # Returns
    /// Provider-neutral session activity response.
    ///
    /// # Errors
    /// Returns a schema or application error when the session cannot be read.
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
        let value = scriberig_app::SessionBackend::read_session(
            self,
            context,
            &scriberig_app::SessionRequest {
                session: session_id,
            },
        )?;
        Ok(session_response(value))
    }

    /// Reads a previously captured model request from an authorized session.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `session`: Serialized session identity.
    /// - `branch`: Serialized branch identity.
    /// - `invocation`: Serialized model invocation identity.
    ///
    /// # Returns
    /// Bounded safe request detail captured before provider serialization.
    ///
    /// # Errors
    /// Returns invalid-request or not-found errors for unavailable records.
    fn model_request_detail(
        &self,
        context: &ApplicationContext,
        session: &str,
        branch: &str,
        invocation: &str,
    ) -> Result<scriberig_protocol::ModelRequestDetailResponse, ApplicationError> {
        server_agent_console::model_request_detail(self, context, session, branch, invocation)
    }

    /// Reads bounded safe tool-result detail from an authorized session.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `session`: Serialized session identity.
    /// - `branch`: Serialized branch identity.
    /// - `result`: Serialized tool result identity.
    ///
    /// # Returns
    /// Safe result detail captured by the trusted tool boundary.
    ///
    /// # Errors
    /// Returns invalid-request or not-found for unavailable records.
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

    /// Selects a model profile for future session invocations.
    ///
    /// # Parameters
    /// - `context`: Trusted capability context.
    /// - `session`: Stable session identity.
    /// - `expected_revision`: Selection revision observed by the caller.
    /// - `model_profile`: Curated profile identity.
    ///
    /// # Returns
    /// Updated session execution selection.
    ///
    /// # Errors
    /// Returns authorization, validation, conflict, or persistence failures.
    fn set_session_model(
        &self,
        context: &ApplicationContext,
        session: &str,
        expected_revision: u64,
        model_profile: &str,
    ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
        server_agent_console::set_session_model(
            self,
            context,
            session,
            expected_revision,
            model_profile,
        )
    }

    /// Selects a thinking level for future session invocations.
    ///
    /// # Parameters
    /// - `context`: Trusted capability context.
    /// - `session`: Stable session identity.
    /// - `expected_revision`: Selection revision observed by the caller.
    /// - `thinking_level`: Advertised thinking capability identity.
    ///
    /// # Returns
    /// Updated session execution selection.
    ///
    /// # Errors
    /// Returns authorization, validation, conflict, or persistence failures.
    fn set_session_thinking_level(
        &self,
        context: &ApplicationContext,
        session: &str,
        expected_revision: u64,
        thinking_level: &str,
    ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
        server_agent_console::set_session_thinking_level(
            self,
            context,
            session,
            expected_revision,
            thinking_level,
        )
    }

    /// Advertises the generic inspect capability supplied by this façade.
    ///
    /// # Returns
    /// One available inspect capability.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        capabilities()
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

    /// Executes a bounded collection listing through the shared application.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-bound list request.
    ///
    /// # Returns
    /// The application-owned list result.
    ///
    /// # Errors
    /// Returns the shared application failure.
    fn list(
        &self,
        context: &ApplicationContext,
        request: &ListRequest,
    ) -> Result<ListResult, ApplicationError> {
        match self.execute(
            context,
            &scriberig_app::ApplicationRequest::List(request.clone()),
        )? {
            scriberig_app::ApplicationResult::List(result) => Ok(result),
            _ => Err(ApplicationError::Internal {
                message: "list backend returned a different result kind".to_owned(),
            }),
        }
    }

    /// Executes semantic discovery through the shared application.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-scoped discovery request.
    ///
    /// # Returns
    /// The application-owned discovery result.
    ///
    /// # Errors
    /// Returns the shared application failure.
    fn find(
        &self,
        context: &ApplicationContext,
        request: &FindRequest,
    ) -> Result<FindResult, ApplicationError> {
        match self.execute(
            context,
            &scriberig_app::ApplicationRequest::Find(request.clone()),
        )? {
            scriberig_app::ApplicationResult::Find(result) => Ok(result),
            _ => Err(ApplicationError::Internal {
                message: "find backend returned a different result kind".to_owned(),
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
            .events_with_sequences()
            .into_iter()
            .map(|(sequence, event)| {
                let mut event = event_from_application(event);
                event.sequence = sequence;
                event
            })
            .filter(|event| event_matches(event, event_kinds))
            .collect())
    }

    /// Returns retained application events after a reconnect cursor.
    ///
    /// # Parameters
    /// - `event_kinds`: Event kind filters.
    /// - `after`: Last cursor already processed by the caller.
    ///
    /// # Returns
    /// Events newer than `after`.
    ///
    /// # Errors
    /// Returns `cursor_too_old` when replay history has expired.
    fn events_after(
        &self,
        event_kinds: &[String],
        after: u64,
    ) -> Result<Vec<Event>, ApplicationError> {
        filter_events_after(self.events(event_kinds)?, after)
    }

    /// Decodes and executes one command through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `kind`: Stable command family.
    /// - `payload`: Typed command payload.
    ///
    /// # Returns
    /// The accepted or completed shared operation.
    ///
    /// # Errors
    /// Returns a structured application failure.
    fn command(
        &self,
        context: &ApplicationContext,
        kind: &str,
        payload: &serde_json::Value,
    ) -> Result<Operation, ApplicationError> {
        super::server_agent_command::execute(self, context, kind, payload)
    }
}
