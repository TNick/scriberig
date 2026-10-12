//! Defines stable handles returned when TUI agent work is accepted.

use scriberig_core::EntityId;

/// Tracks the stable handles returned when agent work is accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentHandle {
    /// Shared operation identity.
    pub operation: scriberig_core::OperationId,
    /// Session receiving the operation.
    pub session: EntityId,
    /// Root or continuation branch, when known immediately.
    pub branch: Option<EntityId>,
}
