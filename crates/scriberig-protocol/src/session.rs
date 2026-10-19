//! Defines compact provider-neutral session observation protocol values.

pub use scriberig_session::{SessionActivityKind, SessionActivityState, ToolActivityKind};

#[path = "session/requests.rs"]
mod requests;
pub use requests::{
    ModelRequestDetailRequest, ModelRequestDetailResponse, ModelRequestMessageDetail,
    ModelRequestToolDetail, SessionReadRequest, ToolResultDetailRequest, ToolResultDetailResponse,
};
#[path = "session/activity.rs"]
mod activity;
pub use activity::{
    SessionActivityItem, SessionBranch, SessionBranchResult, SessionBranchStatus, SessionTurn,
};
#[path = "session/plan.rs"]
mod plan;
pub use plan::{
    SessionContinuationAnchor, SessionCoordinatorClaim, SessionExpectedResult,
    SessionModelAssignment, SessionPlan, SessionPlanAction, SessionPlanAssignmentRequest,
    SessionPlanCreateRequest, SessionPlanDependency, SessionPlanLifecycleRequest, SessionPlanNode,
    SessionPlanNodeDraft, SessionPlanNodeReference, SessionPlanNodeStatus,
    SessionPlanReplanRequest, SessionPlanRevision, SessionPlanStatus, SessionWorkerClaim,
};
#[path = "session/execution.rs"]
mod execution;
pub use execution::{SessionExecutionChange, SessionExecutionSelection, SessionReadResponse};
