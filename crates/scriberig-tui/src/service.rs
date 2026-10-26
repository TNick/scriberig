//! Defines the narrow application-facing port used by TUI semantic views.

#![allow(clippy::map_unwrap_or)]

use crate::effect::Effect;
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, InspectRequest, InspectResult, QueryService,
    SnapshotResolution,
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
#[path = "service_plan_contract_methods.rs"]
mod service_plan_contract_methods;
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
pub(crate) use service_project_agent::ProjectAgentTool;
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

pub use agent_handle::{AgentExecutionFailure, AgentHandle, AgentWorkspace};
pub use service_agent_runner::{AgentCommandRunner, SharedAgentCommandRunner};
pub use service_application::ApplicationAdapter;
#[path = "service_recovery.rs"]
mod service_recovery;
#[path = "service_runtime.rs"]
mod service_runtime;
use service_effects::select_branch_subtree;
pub use service_effects::{TuiOperationService, execute_effect};
pub use service_operation_adapter::{ApplicationOperationAdapter, ExplorerLabels};
pub use service_operations::{execute_operation, execute_operation_effect};
pub use service_project_projection::ProjectProjection;
use service_project_projection::render as render_project_source;
pub use service_recovery::TuiRecoveryService;
pub use service_remote::RemoteAdapter;
pub use service_runtime::TuiRuntimeService;

/// Supplies semantic reads to the TUI without exposing storage internals.
pub trait TuiService: TuiModelRequestService + TuiRecoveryService + TuiRuntimeService {
    /// Returns the optional upstream registry for plugin-supplied components.
    ///
    /// # Returns
    /// A clone of the product's active extension component mailbox.
    fn extension_component_mailbox(&self) -> Option<rpi_extensions::ExtensionStatusMailbox> {
        None
    }

    /// Returns upstream plugin commands authorized by the local product host.
    ///
    /// # Returns
    /// Command metadata that can be shown in the unified palette.
    fn plugin_commands(&self) -> Vec<rpi_extensions::isolated::WorkerCommandSpec> {
        Vec::new()
    }

    /// Executes one upstream plugin command through the injected product runner.
    ///
    /// - `command`: Exact normalized command identity.
    /// - `arguments`: Raw text following the command name.
    ///
    /// # Returns
    /// The plugin's structured command response.
    ///
    /// # Errors
    /// Returns a safe failure when the command is unavailable or rejected.
    fn invoke_plugin_command(
        &self,
        command: &str,
        arguments: &str,
    ) -> Result<serde_json::Value, String> {
        let _ = (command, arguments);
        Err("plugin commands are unavailable".to_owned())
    }

    /// Continues an upstream plugin command using an action from its UI response.
    ///
    /// - `command`: Exact normalized command identity.
    /// - `arguments`: Raw text following the command name.
    /// - `action`: Structured selection or submitted text.
    ///
    /// # Returns
    /// The next structured command response.
    ///
    /// # Errors
    /// Returns a safe failure when the command is unavailable or rejected.
    fn invoke_plugin_command_action(
        &self,
        command: &str,
        arguments: &str,
        action: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let _ = action;
        self.invoke_plugin_command(command, arguments)
    }

    /// Inspects one entity through the shared application contract.
    ///
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// The application inspection result.
    ///
    /// Returns the structured application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError>;

    /// Lists bounded snapshot entities for Explorer navigation.
    ///
    /// - `context`: Application context for the request.
    /// - `entity`: Entity owning the Explorer collection.
    /// - `snapshot`: Exact snapshot context.
    /// - `_context`: Application context for the request.
    /// - `_entity`: Entity owning the Explorer collection.
    /// - `_snapshot`: Exact snapshot context.
    /// - `_continuation`: Optional continuation token.
    ///
    /// Snapshot-bound Explorer rows.
    ///
    /// Returns the structured application listing error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `_context`: Application context for the request.
    /// - `_snapshot`: Exact discovery scope.
    /// - `_query`: User-authored search text.
    ///   A bounded find model.
    ///   Returns the structured discovery error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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

    /// Reads local upstream model property descriptors for one provider model.
    ///
    /// - `_context`: Trusted local authorization context.
    /// - `_provider_id`: Exact upstream provider identity.
    /// - `_model_id`: Provider-native model identity.
    ///
    /// # Returns
    /// Upstream schema-backed properties available to the local TUI.
    /// # Errors
    /// Returns an error when access or the upstream config read fails.
    fn model_properties(
        &self,
        _context: &ApplicationContext,
        _provider_id: &str,
        _model_id: &str,
    ) -> Result<Vec<rpi_model_runtime::ModelPropertyDescriptor>, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model property settings".to_owned(),
        })
    }

    /// Updates one local model property through upstream schema validation.
    ///
    /// - `_context`: Trusted local authorization context.
    /// - `_provider_id`: Exact upstream provider identity.
    /// - `_model_id`: Provider-native model identity.
    /// - `_key`: Provider-declared property key.
    /// - `_value`: User value passed unchanged to upstream validation.
    ///
    /// # Returns
    /// Success after the local live model config accepts the update.
    /// # Errors
    /// Returns an error when authorization or upstream validation fails.
    fn set_model_property(
        &self,
        _context: &ApplicationContext,
        _provider_id: &str,
        _model_id: &str,
        _key: &str,
        _value: serde_json::Value,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model property settings".to_owned(),
        })
    }

    /// Executes one shared application operation when the adapter supports it.
    /// - `_context`: Application context for the operation.
    /// - `_request`: Public operation request.
    ///   The operation result when supported.
    ///   Returns a structured application error when the operation is unavailable
    ///   or rejected by the owning application subsystem.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute_operation(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "tui semantic operation".to_owned(),
        })
    }

    /// Reads one public session tree.
    ///
    /// - `_context`: Application context for the request.
    /// - `_session`: Session identity to read.
    ///
    /// The requested session.
    ///
    /// Returns an unsupported-operation error by default.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session read".to_owned(),
        })
    }

    /// Lists durable sessions available through the shared application.
    ///
    /// - `_context`: Caller authorization context.
    ///
    /// Sessions ordered by stable identity.
    ///
    /// Returns an unsupported-operation or storage error when unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when listing fails.
    fn list_sessions(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<Session>, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session listing".to_owned(),
        })
    }

    /// Removes one durable session through the shared application boundary.
    ///
    /// - `_context`: Caller authorization context.
    /// - `_session`: Exact session identity to remove.
    ///
    /// Unit after the session is removed.
    ///
    /// Returns a structured authorization, active-work, or storage error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when removal is rejected or fails.
    fn remove_session(
        &self,
        _context: &ApplicationContext,
        _session: EntityId,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session removal".to_owned(),
        })
    }

    tui_plan_methods!();

    tui_agent_methods!();
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
