//! Adapts the shared application façade to the protocol server boundary.

use super::server_commands::decode_command;
use super::server_helpers::{
    event_from_application, event_matches, operation_from_record, result_handles, session_response,
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
    #[allow(clippy::too_many_lines)]
    fn command(
        &self,
        context: &ApplicationContext,
        kind: &str,
        payload: &serde_json::Value,
    ) -> Result<Operation, ApplicationError> {
        if matches!(kind, "agent_start" | "agent_continue") {
            let session = payload
                .get("session")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ApplicationError::InvalidRequest {
                    operation: kind.to_owned(),
                    message: "session is required".to_owned(),
                })?
                .parse::<scriberig_core::EntityId>()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: kind.to_owned(),
                    message: format!("invalid session: {error}"),
                })?;
            let branch = payload
                .get("branch")
                .and_then(serde_json::Value::as_str)
                .map(str::parse::<scriberig_core::EntityId>)
                .transpose()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: kind.to_owned(),
                    message: format!("invalid branch: {error}"),
                })?;
            let prompt = payload
                .get("prompt")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| ApplicationError::InvalidRequest {
                    operation: kind.to_owned(),
                    message: "prompt is required".to_owned(),
                })?;
            let selection = match payload
                .get("selection")
                .filter(|value| !value.is_null())
                .cloned()
            {
                None => None,
                Some(value) => {
                    let selection =
                        serde_json::from_value::<scriberig_protocol::AgentSelection>(value)
                            .map_err(|error| ApplicationError::InvalidRequest {
                                operation: kind.to_owned(),
                                message: format!("invalid semantic selection: {error}"),
                            })?;
                    Some(scriberig_app::AgentSelection {
                        project: selection
                            .project_id
                            .as_deref()
                            .map(str::parse)
                            .transpose()
                            .map_err(|error| ApplicationError::InvalidRequest {
                                operation: kind.to_owned(),
                                message: format!("invalid selected project: {error}"),
                            })?,
                        root: selection
                            .root
                            .as_ref()
                            .map(|root| root.id.parse())
                            .transpose()
                            .map_err(|error| ApplicationError::InvalidRequest {
                                operation: kind.to_owned(),
                                message: format!("invalid selected root: {error}"),
                            })?
                            .unwrap_or(selection.entity.id.parse().map_err(|error| {
                                ApplicationError::InvalidRequest {
                                    operation: kind.to_owned(),
                                    message: format!("invalid selected entity: {error}"),
                                }
                            })?),
                        entity: selection.entity.id.parse().map_err(|error| {
                            ApplicationError::InvalidRequest {
                                operation: kind.to_owned(),
                                message: format!("invalid selected entity: {error}"),
                            }
                        })?,
                        snapshot: selection.snapshot.id.parse().map_err(|error| {
                            ApplicationError::InvalidRequest {
                                operation: kind.to_owned(),
                                message: format!("invalid selected snapshot: {error}"),
                            }
                        })?,
                    })
                }
            };
            let operation = scriberig_app::SessionBackend::submit_agent_command(
                self,
                context,
                &scriberig_app::AgentCommandRequest {
                    session,
                    branch,
                    prompt: prompt.to_owned(),
                    selection,
                },
            )?;
            return self.operation_status(&operation.to_string());
        }
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
                let _ = self.operations().fail(&operation_id, error.clone());
                Err(error)
            }
        }
    }
}
