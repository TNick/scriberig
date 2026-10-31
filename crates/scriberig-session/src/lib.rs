//! Defines provider-neutral session entities and progressive views.
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod branch;
pub mod branch_result;
pub mod description;
pub mod error;
pub mod execution;
pub mod planning;
pub mod schema;
pub mod turn;
pub mod types;
pub mod views;

pub use branch_result::BranchResult;

pub mod repository;

pub use branch::{Branch, BranchStatus};
pub use description::Description;
pub use error::SessionError;
pub use execution::{
    AgentActivityKind, AgentActivityState, CacheDisposition, ExecutionActivity,
    ModelContextRevisionRef, ModelInvocation, ModelOutput, ModelRequestDetailMessage,
    ModelRequestDetailPart, ModelRequestDetailTool, ModelRequestMessage, ModelRequestProjection,
    ModelRequestTool, ModelRequestToolCall, SelectionError, SessionActivityKind,
    SessionActivityState, SessionExecutionChange, SessionExecutionSelection, ToolActivityKind,
};
pub use planning::{
    CoordinatorClaim, ExpectedResult, ModelAssignment, Plan, PlanAbandonReason, PlanAttemptView,
    PlanDependency, PlanError, PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner,
    PlanEvidenceReference, PlanEvidenceRole, PlanGate, PlanGateCounts, PlanGateEvaluation,
    PlanGatePhase, PlanGateRequirement, PlanGateStatus, PlanGateWaiver, PlanNode, PlanNodeParent,
    PlanNodeRollup, PlanNodeRollupIndex, PlanNodeStatus, PlanNodeStatusCounts, PlanNodeSummary,
    PlanNodeView, PlanRevision, PlanSemanticAttempt, PlanSemanticNodeDetails,
    PlanSemanticPredicate, PlanSemanticReference, PlanStatus, PlanView, WorkerClaim,
    semantic::{
        PlanSemanticError, PlanSemanticHistory, PlanSemanticNodeRef, PlanSemanticRevision,
        PlanSemanticSnapshot,
    },
};
pub use repository::{PlanStoreError, SelectionStoreError, SessionStore};
pub use turn::{
    AgentOutput, ContinuationAnchor, ContinuationOrigin, ContinuationProvenance, HumanInput,
    ResolvedContinuationAnchor, ToolInvocation, ToolResult, Turn,
};
pub use types::{
    BranchCandidateContext, Decision, Goal, LegacyExecutionRecord, LegacyExecutionTarget,
    Observation, Question, Session, Summary, Task,
};
pub use views::{BranchDescriptor, ChildBranchSummary, DecisionEntry, SessionViews, TurnView};
