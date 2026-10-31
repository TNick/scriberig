//! Defines the backend contract for shared application operations.

use super::{
    ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult, EntityId,
    MigrationDetail,
};

/// Provides semantic execution for the universal application request model.
pub trait ApplicationBackend {
    /// Reads canonical detail for one migration proposal.
    ///
    /// - `_context`: Authorized application context.
    /// - `_id`: Stable migration handle.
    ///
    /// Snapshot-bound migration detail.
    ///
    /// Returns unsupported-operation when proposal reads are not configured.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn migration_detail(
        &self,
        _context: &ApplicationContext,
        _id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "migration detail".to_owned(),
        })
    }

    /// Executes one transport-independent application request.
    ///
    /// - `self`: Backend owning semantic subsystem providers.
    /// - `context`: Caller, capability, and request context.
    /// - `request`: Universal application request to execute.
    ///
    /// Structured application result owned by the canonical subsystem.
    ///
    /// Returns a structured application failure without transport details.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError>;

    /// Executes one request with cooperative cancellation checkpoints.
    ///
    /// Backends should override this for operations with multiple internal
    /// stages. The default observes cancellation before and after the ordinary
    /// synchronous operation.
    ///
    /// - `self`: Backend owning semantic subsystem providers.
    /// - `context`: Caller, capability, and request context.
    /// - `request`: Universal application request to execute.
    /// - `cancelled`: Caller-owned cancellation flag.
    ///
    /// The structured application result when execution completes.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns `OperationCancelled` when cancellation is observed, or the
    /// backend's structured application failure.
    fn execute_cancellable(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ApplicationResult, ApplicationError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ApplicationError::OperationCancelled {
                operation: "application request".to_owned(),
            });
        }
        let result = self.execute(context, request)?;
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ApplicationError::OperationCancelled {
                operation: "application request".to_owned(),
            });
        }
        Ok(result)
    }

    /// Resolves the current accepted snapshot for the request.
    ///
    /// - `self`: Backend owning snapshot/root state.
    /// - `_context`: Caller context.
    ///
    /// The exact current snapshot.
    ///
    /// Returns an unsupported-operation error when no current root exists.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn current_snapshot(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_core::SnapshotId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "current snapshot resolution".to_owned(),
        })
    }

    /// Reads a session through the owning application subsystem.
    ///
    /// - `_context`: Application context for the request.
    /// - `_request`: Session request to resolve.
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
        _request: &crate::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session read".to_owned(),
        })
    }

    /// Reads a compact session projection through the owning subsystem.
    ///
    /// Backends with semantic storage should omit unbounded plan history,
    /// gates, and evidence, retaining their bounded summary fields. The
    /// default preserves compatibility by delegating to the complete read.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Stable session identity to resolve.
    ///
    /// # Returns
    /// The compact session projection, or a complete session from the default.
    ///
    /// # Errors
    /// Returns an unsupported-operation or backend read error.
    fn read_session_projection(
        &self,
        context: &ApplicationContext,
        request: &crate::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        self.read_session(context, request)
    }

    /// Atomically persists a session execution selection through its owner.
    ///
    /// - `_context`: Authorized application context.
    /// - `_session`: Session snapshot with the updated selection.
    /// - `_expected_revision`: Selection revision observed by the caller; the
    ///   owner must compare it atomically with the persisted revision.
    ///
    /// Unit after the update is persisted.
    ///
    /// Returns conflict for a stale revision, or unsupported-operation when
    /// the backend cannot atomically mutate session state.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn update_session_execution_selection(
        &self,
        _context: &ApplicationContext,
        _session: &scriberig_session::Session,
        _expected_revision: u64,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session execution selection update".to_owned(),
        })
    }

    /// Starts an agent command through the owning application subsystem.
    ///
    /// - `_context`: Application context for the request.
    /// - `_request`: Agent command request to submit.
    ///
    /// The accepted operation identity.
    ///
    /// Returns an unsupported-operation error by default.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _request: &crate::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "agent command submission".to_owned(),
        })
    }

    /// Persists one complete changed session snapshot through its owning backend.
    ///
    /// - `_context`: Authorized application request context.
    /// - `_session`: Complete provider-neutral session snapshot.
    ///
    /// Returns unsupported-operation when the backend has no session mutation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn save_session(
        &self,
        _context: &ApplicationContext,
        _session: &scriberig_session::Session,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session mutation".to_owned(),
        })
    }

    /// Atomically replaces one plan at its observed durable revision.
    /// Newly settled worker results are committed on their exact branches with
    /// the semantic plan revision; unrelated branch records remain untouched.
    ///
    /// - `_context`: Authorized application context.
    /// - `_session`: Complete snapshot containing the proposed plan.
    /// - `_plan`: Stable plan identity.
    /// - `_expected_revision`: Plan revision observed by the caller.
    ///
    /// Returns unsupported-operation when atomic plan updates are unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap_plan(
        &self,
        _context: &ApplicationContext,
        _session: &scriberig_session::Session,
        _plan: EntityId,
        _expected_revision: u64,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "atomic plan update".to_owned(),
        })
    }
}
