//! Defines compact provider-neutral session observation protocol values.

pub use scriberig_session::{SessionActivityKind, SessionActivityState, ToolActivityKind};

#[path = "session/attempts.rs"]
mod attempts;
#[path = "session/plan_details.rs"]
mod plan_details;
#[path = "session/requests.rs"]
mod requests;
pub use attempts::{SessionPlanAttempt, SessionPlanAttemptsRequest, SessionPlanAttemptsResponse};
pub use plan_details::{SessionPlanDetailsRequest, SessionPlanDetailsResponse};
#[path = "session/plan_view.rs"]
mod plan_view;
pub use plan_view::{
    SessionPlanGraphView, SessionPlanNodeView, SessionPlanView, SessionPlanViewRequest,
};
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
    SessionPlanLifecycleRequest, SessionPlanMutation, SessionPlanMutationRequest, SessionPlanNode,
    SessionPlanNodeDraft, SessionPlanNodeReference, SessionPlanNodeStatus,
    SessionPlanReplanRequest, SessionPlanRevision, SessionPlanSemanticPredicate, SessionPlanStatus,
    SessionPlanStatusCounts, SessionPlanSummary, SessionSemanticReference, SessionWorkerClaim,
};
#[path = "session/execution.rs"]
mod execution;
pub use execution::{SessionExecutionChange, SessionExecutionSelection, SessionReadResponse};
