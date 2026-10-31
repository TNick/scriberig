//! Implements one responsibility of durable plan lifecycle behavior.

use super::{EntityId, Plan, PlanError, PlanGatePhase, PlanNodeStatus, PlanStatus};

impl Plan {
    /// Retries a failed node after verifying its dependencies remain satisfied.
    ///
    /// The previous branch remains in plan revision history as attempt
    /// provenance. The retry clears the current branch binding and advances
    /// the node claim generation only when it is claimed again.
    ///
    /// # Parameters
    /// - `self`: Mutable plan containing the failed node.
    /// - `node`: Stable identity of the failed node to retry.
    /// - `results`: Exact successful branch results available for dependency
    ///   validation.
    ///
    /// # Returns
    /// Unit after the node becomes ready for a new attempt.
    ///
    /// # Errors
    /// Returns `MissingNode`, `InvalidTransition`, `TerminalPlan`, or
    /// `RevisionExhausted` when retry is not safe or the revision cannot
    /// advance.
    pub fn retry_failed_node(
        &mut self,
        node: EntityId,
        results: &[crate::BranchResult],
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if !matches!(self.status, PlanStatus::Running | PlanStatus::Paused)
            || self.revision == u64::MAX
        {
            return Err(if self.revision == u64::MAX {
                PlanError::RevisionExhausted
            } else {
                PlanError::InvalidTransition
            });
        }
        let index = self
            .nodes
            .iter()
            .position(|candidate| candidate.id == node)
            .ok_or(PlanError::MissingNode)?;
        let target = &self.nodes[index];
        if target.status != PlanNodeStatus::Failed
            || target.claim.is_some()
            || target.branch.is_none()
            || !self.dependencies_satisfied(node, results)
            || !self.node_gates_satisfied(node, PlanGatePhase::Entry)
        {
            return Err(PlanError::InvalidTransition);
        }
        let mut proposed = self.clone();
        proposed.nodes[index].status = PlanNodeStatus::Ready;
        proposed.nodes[index].branch = None;
        proposed.nodes[index].result = None;
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Recovers a failed plan into a paused state for explicit repair.
    ///
    /// Active coordinator or worker claims must be settled before recovery.
    /// The plan stays paused until a caller explicitly resumes it.
    ///
    /// # Parameters
    /// - `self`: Failed plan to recover.
    ///
    /// # Returns
    /// Unit after the plan enters `Paused`.
    ///
    /// # Errors
    /// Returns `InvalidTransition` when the plan is not failed or still has
    /// active claims, and `RevisionExhausted` when no next revision exists.
    pub fn recover_failed_plan(&mut self) -> Result<(), PlanError> {
        if self.status != PlanStatus::Failed
            || self.coordinator_claim.is_some()
            || self.revision == u64::MAX
            || self
                .nodes
                .iter()
                .any(|node| node.status == PlanNodeStatus::Running || node.claim.is_some())
        {
            return Err(if self.revision == u64::MAX {
                PlanError::RevisionExhausted
            } else {
                PlanError::InvalidTransition
            });
        }
        let mut proposed = self.clone();
        proposed.status = PlanStatus::Paused;
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Calculates scheduler-ready nodes from exact durable branch results.
    ///
    /// - `self`: Plan whose dependencies are evaluated.
    /// - `results`: Durable branch results available in the session snapshot.
    ///
    /// Stable plan-node identities whose dependency contracts are satisfied.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn ready_nodes_from_results(&self, results: &[crate::BranchResult]) -> Vec<EntityId> {
        let mut ready = self
            .nodes
            .iter()
            .filter(|node| {
                if !matches!(node.status, PlanNodeStatus::Planned | PlanNodeStatus::Ready)
                    || node.branch.is_some()
                {
                    return false;
                }
                self.dependencies_satisfied(node.id, results)
                    && self.node_gates_satisfied(node.id, PlanGatePhase::Entry)
            })
            .map(|node| node.id)
            .collect::<Vec<_>>();
        ready.sort();
        ready
    }

    /// Checks exact successful result contracts for one node's prerequisites.
    ///
    /// # Parameters
    /// - `self`: Plan defining dependency and result contracts.
    /// - `node`: Dependent node being checked.
    /// - `results`: Durable branch results visible to the plan operation.
    ///
    /// # Returns
    /// `true` when every prerequisite points to its exact completed result.
    pub(super) fn dependencies_satisfied(
        &self,
        node: EntityId,
        results: &[crate::BranchResult],
    ) -> bool {
        self.dependencies
            .iter()
            .filter(|edge| edge.dependent == node)
            .all(|edge| self.dependency_satisfied(edge, results))
    }

    /// Validates one exact successful result against its prerequisite contract.
    ///
    /// # Parameters
    /// - `self`: Plan defining prerequisite state and result policy.
    /// - `edge`: Dependency edge to evaluate.
    /// - `results`: Durable results in the same session snapshot.
    ///
    /// `true` when the edge points to the prerequisite's accepted result.
    ///
    /// # Returns
    /// The value described by the return type.
    pub(super) fn dependency_satisfied(
        &self,
        edge: &super::PlanDependency,
        results: &[crate::BranchResult],
    ) -> bool {
        let Some(result_id) = edge.fulfilled_by else {
            return false;
        };
        let Some(result) = results.iter().find(|item| item.id == result_id) else {
            return false;
        };
        let Some(prerequisite) = self.nodes.iter().find(|item| item.id == edge.prerequisite) else {
            return false;
        };
        if result.status != crate::BranchStatus::Completed
            || prerequisite.status != PlanNodeStatus::Completed
            || prerequisite.result != Some(result_id)
            || prerequisite.branch != Some(result.branch)
        {
            return false;
        }
        prerequisite
            .expected_result
            .as_ref()
            .is_none_or(|contract| {
                result.result_kind.as_deref() == Some(contract.kind.as_str())
                    && (!contract.requires_handle
                        || !result.result_handles.is_empty()
                        || !result.references.is_empty())
            })
    }
}
