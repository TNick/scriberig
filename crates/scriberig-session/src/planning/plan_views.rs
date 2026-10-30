//! Assembles bounded shared plan and node projections.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, RevisionId, SnapshotId};

use super::semantic::{PlanSemanticError, PlanSemanticSnapshot};
use super::{
    Plan, PlanEvidenceReference, PlanGatePhase, PlanGateStatus, PlanNodeParent, PlanNodeRollup,
    PlanNodeStatus, PlanNodeStatusCounts, PlanStatus,
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
    pub assignment: Option<super::ModelAssignment>,
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

impl Plan {
    /// Builds a compact shared plan projection from one exact content revision.
    ///
    /// # Parameters
    /// - `self`: Canonical plan projection.
    /// - `snapshot`: Exact universal snapshot, when known by the caller.
    ///
    /// A bounded plan view without per-node evidence/history details.
    ///
    /// Returns a serialization error when a canonical revision cannot be encoded.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn plan_view(&self, snapshot: Option<SnapshotId>) -> Result<PlanView, PlanSemanticError> {
        let semantic = PlanSemanticSnapshot::from_plan(self)?;
        let plan_revision = semantic
            .objects
            .iter()
            .find(|object| object.entity == self.id)
            .ok_or_else(|| invalid_view("plan revision is missing"))?;
        let mut status_counts = PlanNodeStatusCounts::default();
        for node in &self.nodes {
            increment_count(&mut status_counts, node.status);
        }
        Ok(PlanView {
            id: self.id,
            revision: plan_revision.revision,
            snapshot: snapshot.or(self.source_snapshot),
            mutation_revision: self.revision,
            goal: self.goal.clone(),
            status: self.status,
            coordinator: self.coordinator,
            coordinator_cursor: self.coordinator_cursor,
            root_nodes: self.root_nodes.clone(),
            node_count: count_u32(self.nodes.len()),
            status_counts,
        })
    }

    /// Builds one detail projection using exact current results and rollups.
    ///
    /// # Parameters
    /// - `self`: Canonical plan projection.
    /// - `node_id`: Stable `PlanNode` identity to inspect.
    /// - `snapshot`: Exact universal snapshot, when known by the caller.
    /// - `results`: Branch results visible in the same session snapshot, when available.
    /// - `rollups`: Derived rollups computed for the same canonical state.
    ///
    /// A node view with bounded summary and gate/evidence counts.
    ///
    /// Returns an error for unknown nodes, invalid structure, or encoding failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn node_view(
        &self,
        node_id: EntityId,
        snapshot: Option<SnapshotId>,
        results: Option<&[crate::BranchResult]>,
        rollups: &BTreeMap<EntityId, PlanNodeRollup>,
    ) -> Result<PlanNodeView, PlanSemanticError> {
        let node = self
            .nodes
            .iter()
            .find(|candidate| candidate.id == node_id)
            .ok_or_else(|| invalid_view("plan node is missing"))?;
        let semantic = PlanSemanticSnapshot::from_plan(self)?;
        let revision = semantic
            .objects
            .iter()
            .find(|object| object.entity == node_id)
            .ok_or_else(|| invalid_view("plan node revision is missing"))?
            .revision;
        let rollup = rollups
            .get(&node_id)
            .ok_or_else(|| invalid_view("plan node rollup is missing"))?;
        let incoming = self
            .dependencies
            .iter()
            .filter(|edge| edge.dependent == node_id)
            .collect::<Vec<_>>();
        let dependency_blockers = incoming
            .iter()
            .filter(|edge| match results {
                Some(results) => !self.dependency_satisfied(edge, results),
                None => edge.fulfilled_by.is_none(),
            })
            .map(|edge| edge.prerequisite)
            .collect();
        let entry_gates = self.gate_counts(node_id, PlanGatePhase::Entry);
        let completion_gates = self.gate_counts(node_id, PlanGatePhase::Completion);
        let summary = self.summaries.iter().find(|item| item.node == node_id);
        let evidence_count = self
            .evidence
            .iter()
            .filter(|association| match association.owner {
                super::PlanEvidenceOwner::Node(owner) => owner == node_id,
                super::PlanEvidenceOwner::Gate(gate) => self
                    .gates
                    .iter()
                    .any(|item| item.id == gate && item.owner == node_id),
            })
            .count();
        Ok(PlanNodeView {
            id: node.id,
            revision,
            snapshot: snapshot.or(self.source_snapshot),
            plan: self.id,
            parent: node.parent,
            title: node.title.clone(),
            purpose: node.purpose.clone(),
            direct_status: node.status,
            effective_status: rollup.effective_status,
            contains_ignored: rollup.contains_ignored,
            contains_abandoned: rollup.contains_abandoned,
            summary_id: summary.map(|item| item.id),
            summary_preview: summary.map(|item| truncate_summary(&item.text)),
            branch: node.branch,
            result: node.result,
            assignment: node.assignment.clone(),
            dependency_blockers,
            entry_gates,
            completion_gates,
            evidence_count: count_u32(evidence_count),
            can_retry: node.status == PlanNodeStatus::Failed
                && node.claim.is_none()
                && node.branch.is_some()
                && matches!(self.status, PlanStatus::Running | PlanStatus::Paused)
                && results.is_some_and(|results| self.dependencies_satisfied(node_id, results))
                && self.node_gates_satisfied(node_id, PlanGatePhase::Entry),
            last_failure: node.last_failure.clone(),
        })
    }

    /// Counts gates attached to a node by their current evaluated state.
    ///
    /// # Parameters
    /// - `self`: Canonical plan with gate definitions and evidence.
    /// - `node`: Stable owner node identity.
    /// - `phase`: Entry or completion gate phase.
    ///
    /// Stable counts by gate result.
    /// # Returns
    /// Counts of gates in each evaluated state.
    fn gate_counts(&self, node: EntityId, phase: PlanGatePhase) -> PlanGateCounts {
        let mut counts = PlanGateCounts::default();
        for evaluation in self.evaluate_node_gates(node, phase) {
            match evaluation.status {
                PlanGateStatus::Pending => counts.pending = counts.pending.saturating_add(1),
                PlanGateStatus::Satisfied => {
                    counts.satisfied = counts.satisfied.saturating_add(1);
                }
                PlanGateStatus::Failed => counts.failed = counts.failed.saturating_add(1),
                PlanGateStatus::Waived => counts.waived = counts.waived.saturating_add(1),
            }
        }
        counts
    }
}

/// Counts one direct node state without overflowing the compact view.
///
/// # Parameters
/// - `counts`: Direct status accumulator.
/// - `status`: Canonical `PlanNode` lifecycle status.
fn increment_count(counts: &mut PlanNodeStatusCounts, status: PlanNodeStatus) {
    let count = match status {
        PlanNodeStatus::Planned => &mut counts.planned,
        PlanNodeStatus::Ready => &mut counts.ready,
        PlanNodeStatus::Running => &mut counts.running,
        PlanNodeStatus::Completed => &mut counts.completed,
        PlanNodeStatus::Failed => &mut counts.failed,
        PlanNodeStatus::Abandoned => &mut counts.abandoned,
    };
    *count = count.saturating_add(1);
}

/// Converts an unbounded collection length to a bounded view count.
///
/// # Parameters
/// - `count`: Collection size to convert.
///
/// The value clamped to the public `u32` range.
/// # Returns
/// The bounded count.
fn count_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Bounds a summary preview at a Unicode character boundary.
///
/// # Parameters
/// - `summary`: Full safe summary text.
///
/// At most 240 Unicode scalar values with an ellipsis when truncated.
/// # Returns
/// The bounded summary preview.
fn truncate_summary(summary: &str) -> String {
    let mut chars = summary.chars();
    let preview = chars.by_ref().take(240).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

/// Creates a bounded semantic view assembly error.
///
/// # Parameters
/// - `message`: Safe explanation of the invalid source projection.
///
/// A view error.
/// # Returns
/// The typed invalid-object error.
fn invalid_view(message: &str) -> PlanSemanticError {
    PlanSemanticError::InvalidObject(message.to_owned())
}
