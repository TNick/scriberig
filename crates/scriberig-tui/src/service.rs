//! Defines the narrow application-facing port used by TUI semantic views.

#![allow(clippy::map_unwrap_or)]

use crate::{action::Action, effect::Effect, inspector::InspectorModel};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, InspectRequest, InspectResult, QueryService,
    RuntimeCapability, SnapshotResolution,
};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use std::str::FromStr;

#[path = "service_application.rs"]
mod service_application;
#[path = "service_operation_adapter.rs"]
mod service_operation_adapter;
#[path = "service_operations.rs"]
mod service_operations;
#[path = "service_remote.rs"]
mod service_remote;

pub use service_application::ApplicationAdapter;
pub use service_operation_adapter::ApplicationOperationAdapter;
pub use service_operations::{execute_operation, execute_operation_effect};
pub use service_remote::RemoteAdapter;

/// Supplies semantic reads to the TUI without exposing storage internals.
pub trait TuiService {
    /// Inspects one entity through the shared application contract.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The application inspection result.
    ///
    /// # Errors
    /// Returns the structured application error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError>;

    /// Returns runtime capabilities exposed by the application composition.
    ///
    /// # Parameters
    /// - `_context`: Application context for capability resolution.
    ///
    /// # Returns
    /// The capabilities available to the TUI.
    ///
    /// # Errors
    /// Returns the structured application error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(Vec::new())
    }

    /// Executes one shared application operation when the adapter supports it.
    ///
    /// # Parameters
    /// - `_context`: Application context for the operation.
    /// - `_request`: Public operation request.
    ///
    /// # Returns
    /// The operation result when supported.
    ///
    /// # Errors
    /// Returns a structured application error when the operation is unavailable
    /// or rejected by the owning application subsystem.
    fn execute_operation(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "tui semantic operation".to_owned(),
        })
    }

    /// Reads one authoritative operation lifecycle record.
    ///
    /// # Parameters
    /// - `_context`: Application context for the lookup.
    /// - `_operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The current operation record.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    /// - `_context`: Argument named `_context`.
    /// - `operation`: Argument named `operation`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    /// - `_context`: Argument named `_context`.
    /// - `operation`: Argument named `operation`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        _operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation status".to_owned(),
        })
    }

    /// Requests application-owned cancellation of one operation.
    ///
    /// # Parameters
    /// - `_context`: Application context for the cancellation.
    /// - `_operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled operation record.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    /// - `_context`: Argument named `_context`.
    /// - `operation`: Argument named `operation`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    /// - `_context`: Argument named `_context`.
    /// - `operation`: Argument named `operation`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        _operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation cancellation".to_owned(),
        })
    }

    /// Returns application events observed since the previous runtime poll.
    ///
    /// # Parameters
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Retained application events.
    ///
    /// # Errors
    /// Returns an unsupported event-access error when unavailable.
    /// - `_context`: Argument named `_context`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    /// - `_context`: Argument named `_context`.
    ///
    /// # Returns
    /// The value produced by the declaration.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        Ok(Vec::new())
    }

    /// Reads one public session tree.
    ///
    /// # Parameters
    /// - `_context`: Application context for the request.
    /// - `_session`: Session identity to read.
    ///
    /// # Returns
    /// The requested session.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session read".to_owned(),
        })
    }

    /// Submits agent text to a public session/branch operation.
    ///
    /// # Parameters
    /// - `_context`: Application context for the request.
    /// - `_session`: Session identity receiving the prompt.
    /// - `_branch`: Optional branch identity.
    /// - `_prompt`: Agent prompt to submit.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _session: EntityId,
        _branch: Option<EntityId>,
        _prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "agent command submission".to_owned(),
        })
    }
}

/// Supplies shared application operations to semantic TUI views.
pub trait TuiOperationService {
    /// Executes one public application operation without exposing storage.
    ///
    /// # Parameters
    /// - `context`: Authorized application context.
    /// - `request`: Public universal application request.
    ///
    /// # Returns
    /// Structured application result owned by the application layer.
    ///
    /// # Errors
    /// Returns the structured application error from the owning subsystem.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError>;
}

/// Adapts the complete public application facade for operation-backed views.
pub fn execute_effect<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Action {
    match effect {
        Effect::InspectEntity {
            request_id,
            entity,
            snapshot,
        } => match service.inspect_entity(context, entity, snapshot) {
            Ok(result) => Action::InspectSucceeded {
                request_id,
                model: InspectorModel::from_result(result),
            },
            Err(error) => Action::InspectFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::RequestCapabilities => match service.runtime_capabilities(context) {
            Ok(capabilities) => Action::CapabilitiesLoaded(capabilities),
            Err(error) => Action::InspectFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::History { .. }
        | Effect::Diff { .. }
        | Effect::Render { .. }
        | Effect::Apply { .. }
        | Effect::Commit { .. } => execute_operation_effect(service, context, effect),
        Effect::OperationStatus { operation } => service
            .operation_status(context, &operation)
            .map(|record| {
                Action::OperationObserved(crate::view::operations::OperationModel::from_record(
                    &record,
                ))
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::CancelOperation { operation } => service
            .cancel_operation(context, &operation)
            .map(|record| {
                Action::OperationObserved(crate::view::operations::OperationModel::from_record(
                    &record,
                ))
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ReadSession {
            request_id,
            session,
        } => service
            .read_session(context, session)
            .map(|value| Action::SessionSucceeded {
                request_id,
                model: crate::view::session::SessionModel::from_session(&value),
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::SubmitAgent {
            session,
            branch,
            prompt,
        } => service
            .submit_agent_command(context, session, branch, &prompt)
            .map(Action::AgentSubmitted)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::RequestRefresh => Action::RefreshRequested,
        Effect::RequestQuit => Action::QuitRequested,
        Effect::RequestFind { text } => Action::OperationFailed {
            request_id: 0,
            category: "unsupported".to_owned(),
            message: format!("find is not available through this TUI service: {text}"),
        },
        Effect::RequestSemanticAction { id, .. } => Action::OperationFailed {
            request_id: 0,
            category: "unsupported".to_owned(),
            message: format!("semantic action is not available through this TUI service: {id}"),
        },
        Effect::None => Action::Noop,
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
