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
#[path = "service_agent_runner.rs"]
mod service_agent_runner;
#[path = "service_application.rs"]
mod service_application;
#[path = "service_application_model_request.rs"]
mod service_application_model_request;
#[path = "service_effects.rs"]
mod service_effects;
#[path = "service_operation_adapter.rs"]
mod service_operation_adapter;
#[path = "service_operation_agent.rs"]
mod service_operation_agent;
#[path = "service_operation_helpers.rs"]
mod service_operation_helpers;
#[path = "service_operation_model_request.rs"]
mod service_operation_model_request;
#[macro_use]
#[path = "service_contract_methods.rs"]
mod service_contract_methods;
#[path = "service_model_request.rs"]
mod service_model_request;
#[path = "service_operations.rs"]
mod service_operations;
pub use service_model_request::TuiModelRequestService;
#[path = "service_project_agent.rs"]
mod service_project_agent;
#[path = "service_project_candidate_review.rs"]
mod service_project_candidate_review;
#[path = "service_project_projection.rs"]
mod service_project_projection;
pub(crate) use service_project_agent::{ProjectAgentTool, golden_provider};
#[path = "service_remote.rs"]
mod service_remote;
#[path = "service_remote_agent.rs"]
mod service_remote_agent;
#[path = "service_remote_mapping.rs"]
mod service_remote_mapping;
#[path = "service_remote_model_request.rs"]
mod service_remote_model_request;
#[path = "service_remote_models.rs"]
mod service_remote_models;

pub use agent_handle::{AgentHandle, AgentWorkspace};
pub use service_agent_runner::{AgentCommandRunner, SharedAgentCommandRunner};
pub use service_application::ApplicationAdapter;
use service_effects::select_branch_subtree;
pub use service_effects::{TuiOperationService, execute_effect};
pub use service_operation_adapter::{ApplicationOperationAdapter, ExplorerLabels};
pub use service_operations::{execute_operation, execute_operation_effect};
pub use service_project_projection::ProjectProjection;
use service_project_projection::render as render_project_source;
pub use service_remote::RemoteAdapter;

/// Supplies semantic reads to the TUI without exposing storage internals.
pub trait TuiService: TuiModelRequestService {
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
    /// # Parameters
    /// - `_context`: Application context for the request.
    /// - `_snapshot`: Exact discovery scope.
    /// - `_query`: User-authored search text.
    /// # Returns
    /// A bounded find model.
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
    /// # Parameters
    /// - `_context`: Application context for capability resolution.
    /// # Returns
    /// The capabilities available to the TUI.
    /// # Errors
    /// Returns the structured application error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(Vec::new())
    }

    /// Executes one shared application operation when the adapter supports it.
    /// # Parameters
    /// - `_context`: Application context for the operation.
    /// - `_request`: Public operation request.
    /// # Returns
    /// The operation result when supported.
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

    /// Returns a snapshot of the application's tracked operation records.
    ///
    /// # Parameters
    /// - `_context`: Application context for operation visibility.
    ///
    /// # Returns
    /// Current operation records available to this service.
    ///
    /// # Errors
    /// Returns an operation access error when the service cannot list records.
    fn operation_records(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::OperationRecord>, ApplicationError> {
        Ok(Vec::new())
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

    /// Reads bounded safe detail for one tool result.
    /// # Parameters
    /// - `_context`: Authorized request context.
    /// - `_session`: Session containing the result.
    /// - `_branch`: Branch containing the result.
    /// - `_result`: Stable tool-result identity.
    /// # Returns
    /// Bounded allowlisted result detail.
    /// # Errors
    /// Returns unsupported-operation when unavailable.
    fn tool_result_detail(
        &self,
        _context: &ApplicationContext,
        _session: EntityId,
        _branch: EntityId,
        _result: EntityId,
    ) -> Result<scriberig_protocol::ToolResultDetailResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "tool result detail".to_owned(),
        })
    }

    /// Reads the credential-safe model library for interactive selection.
    ///
    /// # Parameters
    /// - `_context`: Application context for the request.
    ///
    /// # Returns
    /// Safe model and profile descriptors.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    fn model_library(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model library".to_owned(),
        })
    }

    /// Updates a session's future model selection using its observed revision.
    ///
    /// # Parameters
    /// - `_context`: Application context for the mutation.
    /// - `_request`: Session, expected revision, and selected profile.
    ///
    /// # Returns
    /// The authoritative updated execution selection.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    fn set_session_model(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionModelSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session model selection".to_owned(),
        })
    }

    /// Updates a session's future thinking selection using its revision.
    ///
    /// # Parameters
    /// - `_context`: Application context for the mutation.
    /// - `_request`: Session, expected revision, and thinking level.
    ///
    /// # Returns
    /// The authoritative updated execution selection.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    fn set_session_thinking_level(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionThinkingSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session thinking selection".to_owned(),
        })
    }

    tui_agent_methods!();
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
