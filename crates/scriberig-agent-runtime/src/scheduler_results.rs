//! Defines the durable outcome of one concurrent plan worker batch.

use scriberig_core::EntityId;
use scriberig_session::BranchResult;

/// Holds safe model requests produced by one exact plan worker branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerModelRequestEvidence {
    /// Claimed branch that owns these requests.
    pub branch: EntityId,
    /// Bounded requests captured before provider execution.
    pub requests: Vec<scriberig_session::ModelRequestProjection>,
}

/// Carries a worker result with its nonsemantic model-request evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanWorkerExecutionResult {
    /// Durable semantic branch result.
    pub result: BranchResult,
    /// Bounded model requests captured during execution.
    pub model_requests: Vec<scriberig_session::ModelRequestProjection>,
}

/// Summarizes one concurrent claimed-worker batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerBatchResult {
    /// Node identities that completed and linked a durable result.
    pub completed: Vec<EntityId>,
    /// Node identities that failed and remain dependency-blocking.
    pub failed: Vec<EntityId>,
    /// Completed and failed branch results ready for session persistence.
    pub results: Vec<BranchResult>,
    /// Safe model requests to persist on their exact worker branches.
    pub model_request_evidence: Vec<WorkerModelRequestEvidence>,
}
