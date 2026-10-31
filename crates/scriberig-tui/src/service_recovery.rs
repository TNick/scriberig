//! Defines the application port for explicit legacy selection recovery.

use crate::service::ApplicationContext;
use scriberig_app::ApplicationError;
use scriberig_core::EntityId;

/// Supplies authorized legacy recovery reads and revision-checked mappings.
pub trait TuiRecoveryService {
    /// Reads sanitized legacy execution choices retained for operator recovery.
    ///
    /// - `context`: Caller authorization context.
    /// - `session`: Exact session identity to inspect.
    ///
    /// Preserved records with exact source paths and stable targets.
    ///
    /// # Returns
    /// The records retained for operator review.
    ///
    /// # Errors
    /// Returns an authorization, unsupported-operation, or storage error.
    fn legacy_execution_records(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<Vec<scriberig_protocol::LegacyExecutionRecordView>, ApplicationError> {
        let _ = (context, session);
        Err(ApplicationError::UnsupportedOperation {
            operation: "legacy execution recovery inspection".to_owned(),
        })
    }

    /// Maps an exact preserved session choice to an active upstream model.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Exact path, observed revision, and operator-selected model.
    ///
    /// # Returns
    /// Unit after the shared application confirms the mapping.
    ///
    /// # Errors
    /// Returns validation, conflict, authorization, or storage failures.
    fn map_legacy_execution_selection(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::LegacyExecutionMappingRequest,
    ) -> Result<(), ApplicationError> {
        let _ = (context, request);
        Err(ApplicationError::UnsupportedOperation {
            operation: "legacy execution selection mapping".to_owned(),
        })
    }

    /// Maps an exact current plan assignment to an active upstream model.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Exact path, observed plan revision, and selected model.
    ///
    /// # Returns
    /// The updated durable plan.
    ///
    /// # Errors
    /// Returns validation, conflict, authorization, or storage failures.
    fn map_legacy_plan_assignment(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::LegacyPlanAssignmentMappingRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        let _ = (context, request);
        Err(ApplicationError::UnsupportedOperation {
            operation: "legacy plan assignment mapping".to_owned(),
        })
    }
}
