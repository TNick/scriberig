//! Defines compact provider-neutral session observation protocol values.

pub use scriberig_session::{SessionActivityKind, SessionActivityState, ToolActivityKind};

#[path = "session/requests.rs"]
mod requests;
pub use requests::{
    ModelContextRevisionReference, ModelRequestDetailFragment, ModelRequestDetailRequest,
    ModelRequestDetailResponse, ModelRequestMessageDetail, ModelRequestToolDetail,
    SessionReadRequest, ToolResultDetailRequest, ToolResultDetailResponse,
};
#[path = "session/activity.rs"]
mod activity;
pub use activity::{
    SessionActivityItem, SessionBranch, SessionBranchResult, SessionBranchStatus, SessionTurn,
};
#[path = "session/plan.rs"]
mod plan;
#[path = "session/plan_operations.rs"]
mod plan_operations;
#[path = "session/plan_semantics.rs"]
mod plan_semantics;
pub use plan::{
    SessionContinuationAnchor, SessionCoordinatorClaim, SessionExpectedResult,
    SessionModelAssignment, SessionPlan, SessionPlanAbandonReason, SessionPlanAction,
    SessionPlanAssignmentRequest, SessionPlanAttempt, SessionPlanAttemptHistoryPage,
    SessionPlanAttemptHistoryRequest, SessionPlanCreateRequest, SessionPlanDependency,
    SessionPlanEvidenceAssociation, SessionPlanEvidenceKind, SessionPlanEvidenceOwner,
    SessionPlanEvidenceReference, SessionPlanEvidenceRole, SessionPlanGate, SessionPlanGateCounts,
    SessionPlanGatePhase, SessionPlanGateRequirement, SessionPlanGateWaiver,
    SessionPlanLifecycleRequest, SessionPlanNode, SessionPlanNodeDraft, SessionPlanNodeReference,
    SessionPlanNodeStatus, SessionPlanReplanRequest, SessionPlanRetryRequest, SessionPlanRevision,
    SessionPlanSemanticPredicate, SessionPlanStatus, SessionPlanStatusCounts, SessionPlanSummary,
    SessionSemanticReference, SessionWorkerClaim,
};
pub use plan_operations::{
    SessionPlanChildDraft, SessionPlanDecomposeRequest, SessionPlanNextStepRequest,
    SessionPlanSemanticMutation, SessionPlanSemanticMutationRequest,
};
#[path = "session/execution.rs"]
mod execution;
pub use execution::{SessionExecutionChange, SessionExecutionSelection, SessionReadResponse};
