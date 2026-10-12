//! Defines the narrow application-facing port used by TUI semantic views.

#![allow(clippy::map_unwrap_or)]

use crate::effect::Effect;
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, InspectRequest, InspectResult, QueryService,
    RuntimeCapability, SnapshotResolution,
};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_session::Session;
use std::str::FromStr;

#[path = "agent_handle.rs"]
mod agent_handle;
#[path = "service_application.rs"]
mod service_application;
#[path = "service_effects.rs"]
mod service_effects;
#[path = "service_operation_adapter.rs"]
mod service_operation_adapter;
#[path = "service_operations.rs"]
mod service_operations;
#[path = "service_remote.rs"]
mod service_remote;
#[path = "service_remote_mapping.rs"]
mod service_remote_mapping;

pub use agent_handle::AgentHandle;
pub use service_application::ApplicationAdapter;
use service_effects::select_branch_subtree;
pub use service_effects::{TuiOperationService, execute_effect};
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

    /// Lists bounded snapshot entities for Explorer navigation.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity owning the Explorer collection.
    /// - `snapshot`: Exact snapshot context.
    /// - `_context`: Application context for the request.
    /// - `_entity`: Entity owning the Explorer collection.
    /// - `_snapshot`: Exact snapshot context.
    /// - `_continuation`: Optional continuation token.
    ///
    /// # Returns
    /// Snapshot-bound Explorer rows.
    ///
    /// # Errors
    /// Returns the structured application listing error.
    fn list_explorer(
        &self,
        _context: &ApplicationContext,
        _entity: EntityId,
        _snapshot: SnapshotId,
        _continuation: Option<scriberig_query::ContinuationToken>,
    ) -> Result<crate::explorer::ExplorerPage, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "explorer listing".to_owned(),
        })
    }

    /// Executes snapshot-scoped semantic discovery.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `snapshot`: Exact discovery scope.
    /// - `query`: User-authored search text.
    /// - `_context`: Application context for the request.
    /// - `_snapshot`: Exact discovery scope.
    /// - `_query`: User-authored search text.
    ///
    /// # Returns
    /// A bounded find model.
    ///
    /// # Errors
    /// Returns the structured discovery error.
    fn find_entities(
        &self,
        _context: &ApplicationContext,
        _snapshot: SnapshotId,
        _query: &str,
    ) -> Result<crate::view::find::FindModel, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "semantic find".to_owned(),
        })
    }

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

    /// Reads one branch subtree without requiring unrelated child histories.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `session`: Stable session identity.
    /// - `branch`: Branch whose activity should be loaded.
    ///
    /// # Returns
    /// The selected branch and nested descendants.
    ///
    /// # Errors
    /// Returns the underlying session read failure.
    fn read_session_branch(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
    ) -> Result<Session, ApplicationError> {
        Ok(select_branch_subtree(
            self.read_session(context, session)?,
            branch,
        ))
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

    /// Starts a shared agent operation at the session root.
    ///
    /// # Parameters
    /// - `context`: Application context for the operation.
    /// - `session`: Session receiving the new root branch.
    /// - `prompt`: User-authored agent instruction.
    ///
    /// # Returns
    /// Stable accepted operation identity.
    ///
    /// # Errors
    /// Returns the service's structured agent submission error.
    fn start_agent(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        self.submit_agent_command(context, session, None, prompt)
    }

    /// Continues a shared agent operation on an existing branch.
    ///
    /// # Parameters
    /// - `context`: Application context for the operation.
    /// - `session`: Session containing the branch.
    /// - `branch`: Existing branch receiving the follow-up.
    /// - `prompt`: User-authored follow-up instruction.
    ///
    /// # Returns
    /// Stable accepted operation identity.
    ///
    /// # Errors
    /// Returns the service's structured agent submission error.
    fn continue_agent(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
        prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        self.submit_agent_command(context, session, Some(branch), prompt)
    }

    /// Submits agent work and preserves stable handles available at acceptance.
    ///
    /// # Parameters
    /// - `context`: Application context for the operation.
    /// - `session`: Session receiving the prompt.
    /// - `branch`: Optional continuation branch.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and known branch handles.
    ///
    /// # Errors
    /// Returns the service's structured agent submission error.
    fn submit_agent_handle(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<AgentHandle, ApplicationError> {
        let operation = branch.map_or_else(
            || self.start_agent(context, session, prompt),
            |branch| self.continue_agent(context, session, branch, prompt),
        )?;
        Ok(AgentHandle {
            operation,
            session,
            branch,
        })
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
