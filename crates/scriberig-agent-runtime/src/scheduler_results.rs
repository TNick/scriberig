//! Defines the durable outcome of one concurrent plan worker batch.

use scriberig_core::EntityId;
use scriberig_session::BranchResult;

/// Summarizes one concurrent claimed-worker batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerBatchResult {
    /// Node identities that completed and linked a durable result.
    pub completed: Vec<EntityId>,
    /// Node identities that failed and remain dependency-blocking.
    pub failed: Vec<EntityId>,
    /// Completed and failed branch results ready for session persistence.
    pub results: Vec<BranchResult>,
}
