//! Implements completion and lifecycle transitions for durable plans.

use super::{
    EntityId, ModelAssignment, Plan, PlanError, PlanGatePhase, PlanNodeStatus, PlanRevision,
    PlanStatus,
};

impl Plan {
    /// Completes the plan after every node result is durably available.
    ///
    /// - `self`: Running plan whose work may be complete.
    /// - `results`: Durable branch results in the owning session snapshot.
    ///
    /// `true` after transitioning to Completed, or `false` while work remains.
    ///
    /// Returns a plan lifecycle error when completion cannot be applied.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_if_satisfied(
        &mut self,
        results: &[crate::BranchResult],
    ) -> Result<bool, PlanError> {
        if self.status != PlanStatus::Running {
            return Err(PlanError::InvalidTransition);
        }
        let complete = self.all_nodes_satisfied(results);
        if complete {
            self.transition(PlanStatus::Completed)?;
        }
        Ok(complete)
    }

    /// Checks whether all required nodes have matching durable results.
    /// # Parameters
    /// - `results`: Input for this operation.
    /// # Returns
    /// The operation result.
    pub(super) fn all_nodes_satisfied(&self, results: &[crate::BranchResult]) -> bool {
        !self.nodes.is_empty()
            && self.nodes.iter().all(|node| {
                if node.status != PlanNodeStatus::Completed {
                    return false;
                }
                if !self.node_gates_satisfied(node.id, PlanGatePhase::Completion) {
                    return false;
                }
                let Some(result_id) = node.result else {
                    return false;
                };
                let Some(result) = results.iter().find(|result| result.id == result_id) else {
                    return false;
                };
                if result.status != crate::BranchStatus::Completed
                    || node.branch != Some(result.branch)
                {
                    return false;
                }
                node.expected_result.as_ref().is_none_or(|contract| {
                    result.result_kind.as_deref() == Some(contract.kind.as_str())
                        && (!contract.requires_handle
                            || !result.result_handles.is_empty()
                            || !result.references.is_empty())
                })
            })
    }

    /// Changes the plan lifecycle while rejecting transitions out of terminal states.
    ///
    /// - `self`: Plan to transition.
    /// - `next`: Requested lifecycle state.
    ///
    /// Returns `TerminalPlan` after completion, failure, or cancellation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn transition(&mut self, next: PlanStatus) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        let valid = matches!(
            (self.status, next),
            (
                PlanStatus::Draft,
                PlanStatus::Planned | PlanStatus::Cancelled
            ) | (
                PlanStatus::Planned | PlanStatus::Paused,
                PlanStatus::Running | PlanStatus::Cancelled
            ) | (
                PlanStatus::Running,
                PlanStatus::Paused
                    | PlanStatus::Completed
                    | PlanStatus::Failed
                    | PlanStatus::Cancelled
            )
        );
        if !valid && self.status != next {
            return Err(PlanError::InvalidTransition);
        }
        self.status = next;
        if next != PlanStatus::Running {
            self.coordinator_claim = None;
        }
        self.bump_revision()?;
        Ok(())
    }

    /// Assigns or clears one unstarted plan node's safe model override.
    ///
    /// - `self`: Plan containing the target node.
    /// - `node`: Stable node identity to update.
    /// - `assignment`: Validated model-library assignment, or `None` to inherit.
    ///
    /// Returns `MissingNode`, `InvalidTransition`, or `TerminalPlan` when the
    /// assignment cannot be changed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn set_assignment(
        &mut self,
        node: EntityId,
        assignment: Option<ModelAssignment>,
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        let target = self
            .nodes
            .iter_mut()
            .find(|entry| entry.id == node)
            .ok_or(PlanError::MissingNode)?;
        if matches!(
            target.status,
            PlanNodeStatus::Running | PlanNodeStatus::Completed
        ) || target.branch.is_some()
        {
            return Err(PlanError::InvalidTransition);
        }
        target.assignment = assignment;
        self.bump_revision()?;
        Ok(())
    }

    /// Rejects mutation after a terminal lifecycle state.
    ///
    /// - `self`: Plan to validate.
    ///
    /// Returns `TerminalPlan` when the plan is terminal.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn ensure_mutable(&self) -> Result<(), PlanError> {
        if matches!(
            self.status,
            PlanStatus::Completed | PlanStatus::Failed | PlanStatus::Cancelled
        ) {
            Err(PlanError::TerminalPlan)
        } else {
            Ok(())
        }
    }

    /// Advances the independent plan mutation revision.
    ///
    /// - `self`: Plan whose revision advances.
    ///
    /// Returns `RevisionExhausted` when no next revision is representable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn bump_revision(&mut self) -> Result<(), PlanError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(PlanError::RevisionExhausted)?;
        self.source_snapshot = None;
        self.record_revision();
        Ok(())
    }

    /// Stores an immutable snapshot of the current plan revision.
    ///
    /// - `self`: Plan whose current revision is retained.
    pub(super) fn record_revision(&mut self) {
        self.history.push(PlanRevision {
            revision: self.revision,
            goal: self.goal.clone(),
            status: self.status,
            nodes: self.nodes.clone(),
            dependencies: self.dependencies.clone(),
            summaries: self.summaries.clone(),
            evidence: self.evidence.clone(),
            gates: self.gates.clone(),
            reason: None,
        });
    }

    /// Checks whether the dependency graph reaches a target node.
    ///
    /// - `self`: Dependency graph to inspect.
    /// - `from`: Starting node.
    /// - `target`: Node to search for.
    ///
    /// `true` when a path exists from `from` to `target`.
    /// # Returns
    /// The operation result.
    pub(super) fn reaches(&self, from: EntityId, target: EntityId) -> bool {
        let mut pending = vec![from];
        let mut visited = Vec::new();
        while let Some(current) = pending.pop() {
            if current == target {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);
            pending.extend(
                self.dependencies
                    .iter()
                    .filter(|edge| edge.prerequisite == current)
                    .map(|edge| edge.dependent),
            );
        }
        false
    }

    /// Recomputes readiness from explicit fulfilled result handles.
    ///
    /// - `self`: Plan to refresh.
    pub(super) fn refresh_readiness(&mut self) {
        for index in 0..self.nodes.len() {
            let node_id = self.nodes[index].id;
            let status = self.nodes[index].status;
            if !matches!(status, PlanNodeStatus::Planned | PlanNodeStatus::Ready) {
                continue;
            }
            let blocked = self
                .dependencies
                .iter()
                .any(|edge| edge.dependent == node_id && edge.fulfilled_by.is_none())
                || !self.node_gates_satisfied(node_id, PlanGatePhase::Entry);
            self.nodes[index].status = if blocked {
                PlanNodeStatus::Planned
            } else {
                PlanNodeStatus::Ready
            };
        }
    }
}
