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
#[path = "session/plan_semantics.rs"]
mod plan_semantics;
pub use plan::{
    SessionContinuationAnchor, SessionCoordinatorClaim, SessionExpectedResult,
    SessionModelAssignment, SessionPlan, SessionPlanAbandonReason, SessionPlanAction,
    SessionPlanAssignmentRequest, SessionPlanCreateRequest, SessionPlanDependency,
    SessionPlanEvidenceAssociation, SessionPlanEvidenceKind, SessionPlanEvidenceOwner,
    SessionPlanEvidenceReference, SessionPlanEvidenceRole, SessionPlanGate, SessionPlanGateCounts,
    SessionPlanGatePhase, SessionPlanGateRequirement, SessionPlanGateWaiver,
    SessionPlanLifecycleRequest, SessionPlanNode, SessionPlanNodeDraft, SessionPlanNodeReference,
    SessionPlanNodeStatus, SessionPlanReplanRequest, SessionPlanRevision,
    SessionPlanSemanticPredicate, SessionPlanStatus, SessionPlanStatusCounts, SessionPlanSummary,
    SessionSemanticReference, SessionWorkerClaim,
};
#[path = "session/execution.rs"]
mod execution;
pub use execution::{SessionExecutionChange, SessionExecutionSelection, SessionReadResponse};
