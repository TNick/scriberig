//! Defines public application boundaries for session reads and agent commands.

use scriberig_core::{EntityId, OperationId};
use scriberig_session::Session;

/// Identifies a session read request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionRequest {
    /// Stable session entity identity.
    pub session: EntityId,
}

/// Identifies an agent command submitted to a session branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentCommandRequest {
    /// Target session entity.
    pub session: EntityId,
    /// Optional branch receiving the new turn.
    pub branch: Option<EntityId>,
    /// User-authored command text.
    pub prompt: String,
}

/// Owns session and agent operations at the application boundary.
pub trait SessionBackend {
    /// Reads one public session snapshot.
    ///
    /// # Parameters
    /// - `context`: Authorized caller context.
    /// - `request`: Stable session request.
    ///
    /// # Returns
    /// The session state supplied by the owning subsystem.
    ///
    /// # Errors
    /// Returns a structured application failure.
    fn read_session(
        &self,
        context: &crate::ApplicationContext,
        request: &SessionRequest,
    ) -> Result<Session, crate::ApplicationError>;

    /// Starts one agent command without making its output canonical.
    ///
    /// # Parameters
    /// - `context`: Authorized caller context.
    /// - `request`: Session, branch, and prompt input.
    ///
    /// # Returns
    /// Stable operation identity for lifecycle observation.
    ///
    /// # Errors
    /// Returns a structured application failure.
    fn submit_agent_command(
        &self,
        context: &crate::ApplicationContext,
        request: &AgentCommandRequest,
    ) -> Result<OperationId, crate::ApplicationError>;
}
