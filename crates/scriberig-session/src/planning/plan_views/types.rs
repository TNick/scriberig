//! Defines public plan and node view records.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

use super::super::{
    ModelAssignment, PlanEvidenceReference, PlanNodeParent, PlanNodeStatus, PlanNodeStatusCounts,
    PlanStatus,
};

/// Counts gates by their deterministic evaluated state.
///
/// # Public members
/// - `pending`: Gates awaiting evidence or child completion.
/// - `satisfied`: Gates satisfied by current canonical state.
/// - `failed`: Gates blocked by contradicting or failed evidence.
/// - `waived`: Gates explicitly waived with retained provenance.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanGateCounts {
    /// Gates awaiting evidence or child completion.
    pub pending: u32,
    /// Gates satisfied by current canonical state.
    pub satisfied: u32,
    /// Gates blocked by contradicting or failed evidence.
    pub failed: u32,
    /// Gates explicitly waived with retained provenance.
    pub waived: u32,
}

/// Summarizes one immutable plan graph at an exact content revision.
///
/// # Public members
/// - `id`: Stable plan entity identity.
/// - `revision`: Exact immutable `Plan` revision.
/// - `snapshot`: Exact universal snapshot, when loaded from durable storage.
/// - `mutation_revision`: Plan-local optimistic concurrency revision.
/// - `goal`: Durable plan objective.
/// - `status`: Plan lifecycle state.
/// - `coordinator`: Branch assigned to automatic coordination.
/// - `coordinator_cursor`: Current coordinator node.
/// - `root_nodes`: Structural roots in deterministic membership order.
/// - `node_count`: Number of independently addressable plan nodes.
/// - `status_counts`: Direct node status counts.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanView {
    /// Stable plan entity identity.
    pub id: EntityId,
    /// Exact immutable Plan revision.
    pub revision: RevisionId,
    /// Exact universal snapshot, when loaded from durable storage.
    pub snapshot: Option<SnapshotId>,
    /// Plan-local optimistic concurrency revision.
    pub mutation_revision: u64,
    /// Durable plan objective.
    pub goal: String,
    /// Plan lifecycle state.
    pub status: PlanStatus,
    /// Branch assigned to automatic coordination.
    pub coordinator: Option<EntityId>,
    /// Current coordinator node.
    pub coordinator_cursor: Option<EntityId>,
    /// Structural roots in deterministic membership order.
    pub root_nodes: Vec<EntityId>,
    /// Number of independently addressable plan nodes.
    pub node_count: u32,
    /// Direct node status counts.
    pub status_counts: PlanNodeStatusCounts,
}

/// Projects one node's canonical state and derived blockers for product use.
///
/// # Public members
/// - `id`: Stable `PlanNode` identity.
/// - `revision`: Exact immutable `PlanNode` revision.
/// - `snapshot`: Exact universal snapshot, when loaded from durable storage.
/// - `plan`: Stable owner `Plan` identity.
/// - `parent`: Direct structural parent.
/// - `title`: Human-readable work title.
/// - `purpose`: Focused work instruction.
/// - `direct_status`: Canonical `PlanNode` lifecycle state.
/// - `effective_status`: Derived status including required descendants/gates.
/// - `contains_ignored`: Whether this subtree includes ignored work.
/// - `contains_abandoned`: Whether this subtree includes abandoned work.
/// - `summary_id`: Stable current summary identity.
/// - `summary_preview`: Bounded concise summary text.
/// - `branch`: Current or most recent execution branch.
/// - `result`: Accepted successful result identity.
/// - `assignment`: Safe upstream provider/model assignment.
/// - `dependency_blockers`: Prerequisites not fulfilled by exact accepted results.
/// - `entry_gates`: Entry gate counts by evaluated state.
/// - `completion_gates`: Completion gate counts by evaluated state.
/// - `evidence_count`: Number of node and gate evidence associations.
/// - `can_retry`: Whether current plan state satisfies retry preconditions.
/// - `last_failure`: Exact failed result reference retained after retry.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanNodeView {
    /// Stable `PlanNode` identity.
    pub id: EntityId,
    /// Exact immutable `PlanNode` revision.
    pub revision: RevisionId,
    /// Exact universal snapshot, when loaded from durable storage.
    pub snapshot: Option<SnapshotId>,
    /// Stable owner `Plan` identity.
    pub plan: EntityId,
    /// Direct structural parent.
    pub parent: Option<PlanNodeParent>,
    /// Human-readable work title.
    pub title: String,
    /// Focused work instruction.
    pub purpose: String,
    /// Canonical `PlanNode` lifecycle state.
    pub direct_status: PlanNodeStatus,
    /// Derived status including required descendants/gates.
    pub effective_status: PlanNodeStatus,
    /// Whether this subtree includes ignored work.
    pub contains_ignored: bool,
    /// Whether this subtree includes abandoned work.
    pub contains_abandoned: bool,
    /// Stable current summary identity.
    pub summary_id: Option<EntityId>,
    /// Bounded concise summary text.
    pub summary_preview: Option<String>,
    /// Current or most recent execution branch.
    pub branch: Option<EntityId>,
    /// Accepted successful result identity.
    pub result: Option<EntityId>,
    /// Safe upstream provider/model assignment.
    pub assignment: Option<ModelAssignment>,
    /// Prerequisites not fulfilled by exact accepted results.
    pub dependency_blockers: Vec<EntityId>,
    /// Entry gate counts by evaluated state.
    pub entry_gates: PlanGateCounts,
    /// Completion gate counts by evaluated state.
    pub completion_gates: PlanGateCounts,
    /// Number of node and gate evidence associations.
    pub evidence_count: u32,
    /// Whether current plan state satisfies retry preconditions.
    pub can_retry: bool,
    /// Exact failed result reference retained after retry.
    pub last_failure: Option<PlanEvidenceReference>,
}
