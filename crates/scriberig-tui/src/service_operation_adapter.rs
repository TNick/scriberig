//! Adapts application operations to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Adapts the application backend to operation-oriented TUI services.
pub struct ApplicationOperationAdapter<B> {
    service: ApplicationService<B>,
}

/// Adapts the public protocol client to the TUI inspection boundary.
impl<B> ApplicationOperationAdapter<B> {
    /// Creates an operation adapter around an application service.
    #[must_use]
    pub const fn new(service: ApplicationService<B>) -> Self {
        Self { service }
    }
}

impl<B: ApplicationBackend> TuiService for ApplicationOperationAdapter<B> {
    /// Delegates inspection through the public application facade.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the application inspection error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.service.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }

    /// Delegates shared operations through the public application facade.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Public operation request.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns the application operation error.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        self.service.execute(context, request)
    }

    /// Reads an operation record from the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The current operation record.
    ///
    /// # Errors
    /// Returns a lookup error when the operation is unknown.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        self.service.operations().get(operation)
    }

    /// Cancels an operation through the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for the cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled operation record.
    ///
    /// # Errors
    /// Returns a cancellation error when the operation is unknown.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        self.service.operations().cancel(operation)
    }

    /// Returns lifecycle events retained by the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Retained application events.
    ///
    /// # Errors
    /// This adapter does not return an event access error.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        Ok(self.service.operations().events())
    }

    /// Reads a session through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `session`: Session identity to read.
    ///
    /// # Returns
    /// The requested session.
    ///
    /// # Errors
    /// Returns the application session error.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        scriberig_app::SessionBackend::read_session(
            &self.service,
            context,
            &scriberig_app::SessionRequest { session },
        )
    }

    /// Submits an agent command through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `session`: Session identity receiving the prompt.
    /// - `branch`: Optional branch identity.
    /// - `prompt`: Agent prompt to submit.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns the application agent error.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        scriberig_app::SessionBackend::submit_agent_command(
            &self.service,
            context,
            &scriberig_app::AgentCommandRequest {
                session,
                branch,
                prompt: prompt.to_owned(),
            },
        )
    }
}
