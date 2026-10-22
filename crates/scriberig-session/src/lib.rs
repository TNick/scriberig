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
    use std::collections::BTreeMap;

    use crate::{Plan, PlanNodeRollup, Session};
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

        /// Loads one plan projection from an exact historical semantic snapshot.
        ///
        /// - `_session`: Owning durable session identity.
        /// - `_plan`: Stable plan identity.
        /// - `_snapshot`: Exact universal snapshot to resolve.
        ///
        /// The plan at that snapshot, or `None` when the snapshot is not in
        /// the plan root's history.
        ///
        /// Returns an error when the backend cannot resolve semantic history.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns a backend diagnostic when snapshot resolution fails.
        fn load_plan_at_snapshot(
            &self,
            _session: EntityId,
            _plan: EntityId,
            _snapshot: SnapshotId,
        ) -> Result<Option<Plan>, String> {
            Err("historical plan snapshot reads are unsupported".to_owned())
        }

        /// Loads one compact semantic plan projection without reconstructing history.
        ///
        /// - `_session`: Owning durable session identity.
        /// - `_plan`: Stable plan identity.
        /// - `_snapshot`: Exact snapshot, or `None` for the current plan root.
        ///
        /// The plan projection with an empty compatibility history, or `None`
        /// when the plan or snapshot does not exist.
        ///
        /// Returns a backend diagnostic when the semantic projection cannot
        /// be read.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns a backend diagnostic when snapshot resolution fails.
        fn load_plan_projection(
            &self,
            _session: EntityId,
            _plan: EntityId,
            _snapshot: Option<SnapshotId>,
        ) -> Result<Option<Plan>, String> {
            Err("compact semantic plan reads are unsupported".to_owned())
        }

        /// Loads snapshot-bound derived rollups for compact plan views.
        ///
        /// - `_session`: Owning durable session identity.
        /// - `_plan`: Stable plan identity.
        /// - `_snapshot`: Exact canonical plan snapshot.
        ///
        /// Cached node rollups, or `None` when the backend has no matching
        /// cache.
        ///
        /// Returns a backend diagnostic when a matching cache is corrupt.
        /// # Returns
        /// The current derived summary when available.
        /// # Errors
        /// Returns a backend error when the index cannot be read.
        fn load_plan_rollups(
            &self,
            _session: EntityId,
            _plan: EntityId,
            _snapshot: SnapshotId,
        ) -> Result<Option<BTreeMap<EntityId, PlanNodeRollup>>, String> {
            Ok(None)
        }

        /// Loads branch results without hydrating semantic plan histories.
        ///
        /// - `_session`: Owning durable session identity.
        ///
        /// The session's branch results, or `None` when this optimized read is
        /// unsupported or the session does not exist.
        ///
        /// Returns a backend diagnostic when the stored session is unreadable.
        /// # Returns
        /// The result data required to evaluate plan dependencies and retries.
        /// # Errors
        /// Returns a backend diagnostic when the payload cannot be decoded.
        fn load_plan_branch_results(
            &self,
            _session: EntityId,
        ) -> Result<Option<Vec<crate::BranchResult>>, String> {
            Ok(None)
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
        /// New branch records in the proposal are merged and linked to their
        /// existing parents in the same transaction. Existing branch records
        /// are never overwritten by this operation.
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
    BranchCandidateContext, BranchResult, Decision, Goal, Observation, Question, Session, Summary,
    Task,
};
pub use views::{BranchDescriptor, ChildBranchSummary, DecisionEntry, SessionViews, TurnView};
