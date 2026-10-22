//! Defines compact durable plan graph view contracts.

use crate::{
    SessionPlanGateCounts, SessionPlanNodeStatus, SessionPlanStatus, SessionPlanStatusCounts,
    SessionReference,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests a compact plan graph at the current or exact snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanViewRequest {
    /// Session containing the plan.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Exact semantic snapshot, or current state when absent.
    #[serde(default)]
    pub snapshot: Option<String>,
}

/// Compact plan graph response with exact semantic revision references.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanGraphView {
    /// Compact plan metadata.
    pub plan: SessionPlanView,
    /// Compact node views in stable structural order.
    pub nodes: Vec<SessionPlanNodeView>,
}

/// Compact metadata for one exact plan revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanView {
    /// Stable plan identity.
    pub id: String,
    /// Exact immutable semantic revision.
    pub revision: String,
    /// Exact universal snapshot when available.
    pub snapshot: Option<String>,
    /// Plan-local optimistic concurrency revision.
    pub mutation_revision: u64,
    /// Durable plan objective.
    pub goal: String,
    /// Current lifecycle status.
    pub status: SessionPlanStatus,
    /// Branch assigned to automatic coordination.
    pub coordinator: Option<String>,
    /// Current coordinator node.
    pub coordinator_cursor: Option<String>,
    /// Structural root node identities.
    pub root_nodes: Vec<String>,
    /// Number of independently addressable nodes.
    pub node_count: u32,
    /// Direct node status counts.
    pub status_counts: SessionPlanStatusCounts,
}

/// Compact semantic and execution projection of one plan node.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanNodeView {
    /// Stable node identity.
    pub id: String,
    /// Exact immutable semantic revision.
    pub revision: String,
    /// Exact universal snapshot when available.
    pub snapshot: Option<String>,
    /// Stable owner plan identity.
    pub plan: String,
    /// Parent identity encoded as `branch:<id>` or `node:<id>`.
    pub parent: Option<String>,
    /// Human-readable work title.
    pub title: String,
    /// Focused work instruction.
    pub purpose: String,
    /// Canonical direct lifecycle status.
    pub direct_status: SessionPlanNodeStatus,
    /// Derived status including required descendants and gates.
    pub effective_status: SessionPlanNodeStatus,
    /// Whether this subtree contains ignored work.
    pub contains_ignored: bool,
    /// Whether this subtree contains abandoned work.
    pub contains_abandoned: bool,
    /// Stable summary identity.
    pub summary_id: Option<String>,
    /// Bounded summary preview.
    pub summary_preview: Option<String>,
    /// Current or most recent execution branch.
    pub branch: Option<String>,
    /// Accepted successful result identity.
    pub result: Option<String>,
    /// Safe provider/model assignment.
    pub assignment: Option<super::SessionModelAssignment>,
    /// Prerequisite node identities not yet fulfilled.
    pub dependency_blockers: Vec<String>,
    /// Entry gate counts by evaluated state.
    pub entry_gates: SessionPlanGateCounts,
    /// Completion gate counts by evaluated state.
    pub completion_gates: SessionPlanGateCounts,
    /// Number of associated evidence records.
    pub evidence_count: u32,
    /// Whether retry preconditions are currently satisfied.
    pub can_retry: bool,
    /// Exact failed result reference retained after retry.
    pub last_failure: Option<super::SessionPlanEvidenceReference>,
}
