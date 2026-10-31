//! Assembles bounded shared plan and node projections.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, SnapshotId};

use super::semantic::{PlanSemanticError, PlanSemanticSnapshot};
use super::{
    Plan, PlanGatePhase, PlanGateStatus, PlanNodeRollup, PlanNodeStatus, PlanNodeStatusCounts,
    PlanStatus,
};

mod types;
pub use types::{PlanGateCounts, PlanNodeView, PlanView};

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
        self.plan_view_from_semantic(snapshot, &semantic)
    }

    /// Builds a plan header and all node views from one canonical encoding.
    ///
    /// # Parameters
    /// - `self`: Canonical plan projection.
    /// - `snapshot`: Exact universal snapshot, when known by the caller.
    /// - `results`: Branch results visible in the same session snapshot.
    ///
    /// The shared plan view and node views in stable plan order.
    /// # Returns
    /// The projections for the exact supplied plan state.
    /// # Errors
    /// Returns an error when semantic encoding, structure, or rollups are invalid.
    pub fn semantic_views(
        &self,
        snapshot: Option<SnapshotId>,
        results: &[crate::BranchResult],
    ) -> Result<(PlanView, Vec<PlanNodeView>), PlanSemanticError> {
        let semantic = PlanSemanticSnapshot::from_plan(self)?;
        let plan_view = self.plan_view_from_semantic(snapshot, &semantic)?;
        let rollups = self
            .node_rollups()
            .map_err(|error| invalid_view(format!("plan rollup evaluation failed: {error:?}")))?;
        let nodes = self
            .nodes
            .iter()
            .map(|node| {
                self.node_view_from_semantic(node.id, snapshot, Some(results), &rollups, &semantic)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((plan_view, nodes))
    }

    /// Builds a compact header from a shared semantic encoding.
    ///
    /// - `self`: Canonical plan projection.
    /// - `snapshot`: Exact snapshot requested by the caller.
    /// - `semantic`: Single semantic encoding shared by a view batch.
    ///
    /// The bounded plan header.
    /// # Returns
    /// The plan view with direct node counts.
    /// # Errors
    /// Returns an error if the encoded plan revision is missing.
    fn plan_view_from_semantic(
        &self,
        snapshot: Option<SnapshotId>,
        semantic: &PlanSemanticSnapshot,
    ) -> Result<PlanView, PlanSemanticError> {
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
        let semantic = PlanSemanticSnapshot::from_plan(self)?;
        self.node_view_from_semantic(node_id, snapshot, results, rollups, &semantic)
    }

    /// Builds one node view from a shared semantic encoding.
    ///
    /// - `node_id`: Stable node identity to inspect.
    /// - `snapshot`: Exact snapshot requested by the caller.
    /// - `results`: Branch results visible in the same session snapshot.
    /// - `rollups`: Derived rollups computed for the same canonical state.
    /// - `semantic`: Single semantic encoding shared by a view batch.
    ///
    /// The node view with current status, blockers, and detail counts.
    /// # Returns
    /// The projection for the selected node.
    /// # Errors
    /// Returns an error for an unknown node or missing revision/rollup.
    fn node_view_from_semantic(
        &self,
        node_id: EntityId,
        snapshot: Option<SnapshotId>,
        results: Option<&[crate::BranchResult]>,
        rollups: &BTreeMap<EntityId, PlanNodeRollup>,
        semantic: &PlanSemanticSnapshot,
    ) -> Result<PlanNodeView, PlanSemanticError> {
        let node = self
            .nodes
            .iter()
            .find(|candidate| candidate.id == node_id)
            .ok_or_else(|| invalid_view("plan node is missing"))?;
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
fn invalid_view(message: impl Into<String>) -> PlanSemanticError {
    PlanSemanticError::InvalidObject(message.into())
}
