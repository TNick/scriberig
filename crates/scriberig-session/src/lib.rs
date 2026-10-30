//! Defines provider-neutral session entities and progressive views.
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod branch;
pub mod description;
pub mod error;
pub mod execution;
pub mod planning;
pub mod schema;
pub mod turn;
pub mod types;
pub mod views;

/// Defines the durable session repository boundary.
pub mod repository {
    use crate::{Plan, PlanSemanticAttempt, PlanView, Session};
    use scriberig_core::{EntityId, SnapshotId};

    /// Reports durable execution-selection update failures.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum SelectionStoreError {
        /// The stored selection has moved past the caller's observed revision.
        RevisionConflict { expected: u64, actual: u64 },
        /// The repository could not perform an atomic update.
        Persistence(String),
    }

    /// Reports compare-and-swap plan update failures.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum PlanStoreError {
        /// The stored plan has moved past the caller's observed revision.
        RevisionConflict { expected: u64, actual: u64 },
        /// The repository could not perform an atomic update.
        Persistence(String),
    }

    /// Persists provider-neutral session graphs without depending on a model runtime.
    pub trait SessionStore: Send + Sync {
        /// Lists durable session graphs in deterministic identity order.
        ///
        /// The list is a snapshot of records visible to the backend when the
        /// method is called. Callers should retain exact session identities
        /// and load an individual graph before editing it.
        ///
        /// Complete stored session graphs in deterministic order.
        ///
        /// Returns a bounded backend diagnostic when records cannot be read.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the repository cannot read its records.
        fn list(&self) -> Result<Vec<Session>, String>;

        /// Loads one exact session graph when it exists.
        ///
        /// - `session`: Stable session identity.
        ///
        /// The stored session, or `None` when no record exists.
        ///
        /// Returns a backend diagnostic that is safe for the application layer.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn load(&self, session: EntityId) -> Result<Option<Session>, String>;

        /// Loads one plan from an exact snapshot in that plan's history.
        ///
        /// - `_session`: Owning durable session identity.
        /// - `_plan`: Stable plan identity.
        /// - `_snapshot`: Exact semantic snapshot to resolve.
        ///
        /// The plan projection or `None` when the snapshot is not in the
        /// selected plan's retained history.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns a bounded persistence diagnostic when the snapshot cannot
        /// be read or decoded.
        fn load_plan_at_snapshot(
            &self,
            _session: EntityId,
            _plan: EntityId,
            _snapshot: SnapshotId,
        ) -> Result<Option<Plan>, String> {
            Err("exact plan snapshot reads are unsupported".to_owned())
        }

        /// Loads compact plan metadata at an exact retained snapshot.
        ///
        /// - `_session`: Owning durable session identity.
        /// - `_plan`: Stable plan identity.
        /// - `_snapshot`: Exact semantic snapshot to resolve.
        ///
        /// The compact plan projection or `None` when the snapshot is outside
        /// the selected plan's retained history.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns a bounded persistence diagnostic when the view cannot be
        /// read or assembled.
        fn load_plan_view_at_snapshot(
            &self,
            _session: EntityId,
            _plan: EntityId,
            _snapshot: SnapshotId,
        ) -> Result<Option<PlanView>, String> {
            Err("compact plan snapshot reads are unsupported".to_owned())
        }

        /// Loads one page of worker attempts for a node at an exact snapshot.
        ///
        /// Attempt records are read independently from node evidence and gate
        /// detail so callers can request history only when needed.
        ///
        /// - `_session`: Owning durable session identity.
        /// - `_plan`: Stable plan identity.
        /// - `_node`: Stable plan-node identity.
        /// - `_snapshot`: Exact semantic snapshot to resolve.
        /// - `_offset`: Number of matching attempts to skip in stable order.
        /// - `_limit`: Maximum number of attempts to return.
        ///
        /// The requested page, or `None` when the snapshot is outside the
        /// selected plan's retained history.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns a bounded persistence diagnostic when attempt records cannot
        /// be read or decoded.
        fn load_plan_attempts_at_snapshot(
            &self,
            _session: EntityId,
            _plan: EntityId,
            _node: EntityId,
            _snapshot: SnapshotId,
            _offset: usize,
            _limit: usize,
        ) -> Result<Option<Vec<PlanSemanticAttempt>>, String> {
            Err("plan attempt history reads are unsupported".to_owned())
        }

        /// Removes one exact durable session graph.
        ///
        /// - `_session`: Stable session identity to remove.
        ///
        /// Unit after the session record is removed.
        ///
        /// Returns a backend diagnostic when removal is unavailable or fails.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the repository cannot remove the record.
        fn remove(&self, _session: EntityId) -> Result<(), String> {
            Err("session removal is unsupported".to_owned())
        }

        /// Stores one complete session graph atomically.
        ///
        /// - `session`: Session graph to persist.
        ///
        /// Returns a backend diagnostic when persistence fails.
        ///
        /// Unit after the session is persisted.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn save(&self, session: &Session) -> Result<(), String>;

        /// Atomically updates only execution selection at an observed revision.
        ///
        /// - `_session`: Session carrying the proposed new selection.
        /// - `_expected_revision`: Selection revision observed by the caller.
        ///
        /// Unit after the selection is committed.
        ///
        /// Returns a revision conflict or a repository diagnostic.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn compare_and_swap_execution_selection(
            &self,
            _session: &Session,
            _expected_revision: u64,
        ) -> Result<(), SelectionStoreError> {
            Err(SelectionStoreError::Persistence(
                "atomic session selection updates are unsupported".to_owned(),
            ))
        }

        /// Atomically replaces one plan at its observed revision.
        ///
        /// The repository must preserve unrelated session and branch updates.
        /// New branch records are merged and linked to their parents. Existing
        /// branches are updated only when a prior active worker claim settles;
        /// that result-bearing branch and the plan revision commit together.
        ///
        /// - `_session`: Snapshot containing the proposed plan.
        /// - `_plan`: Stable plan identity being replaced.
        /// - `_expected_revision`: Plan revision observed by the caller.
        ///
        /// Returns a revision conflict or a repository diagnostic.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn compare_and_swap_plan(
            &self,
            _session: &Session,
            _plan: EntityId,
            _expected_revision: u64,
        ) -> Result<(), PlanStoreError> {
            Err(PlanStoreError::Persistence(
                "atomic plan updates are unsupported".to_owned(),
            ))
        }
    }
}

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
    PlanNodeView, PlanRevision, PlanSemanticPredicate, PlanSemanticReference, PlanStatus, PlanView,
    WorkerClaim,
    semantic::{
        PlanSemanticAttempt, PlanSemanticError, PlanSemanticHistory, PlanSemanticNodeRef,
        PlanSemanticRevision, PlanSemanticSnapshot,
    },
};
pub use repository::{PlanStoreError, SelectionStoreError, SessionStore};
pub use turn::{
    AgentOutput, ContinuationAnchor, ContinuationOrigin, ContinuationProvenance, HumanInput,
    ResolvedContinuationAnchor, ToolInvocation, ToolResult, Turn,
};
pub use types::{
    BranchCandidateContext, BranchResult, Decision, Goal, Observation, Question, Session, Summary,
    Task,
};
pub use views::{BranchDescriptor, ChildBranchSummary, DecisionEntry, SessionViews, TurnView};
