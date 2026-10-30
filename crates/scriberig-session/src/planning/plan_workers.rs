//! Implements one responsibility of durable plan lifecycle behavior.

use super::{EntityId, Plan, PlanError, PlanNodeStatus, PlanStatus};

impl Plan {
    /// Lists worker branches settled between two consecutive plan revisions.
    ///
    /// Only prior active claims that now have an exact successful result or
    /// failure reference are returned. The repository uses these identities
    /// to commit branch outcomes with the semantic plan revision.
    ///
    /// - `self`: Proposed plan after worker settlement.
    /// - `previous`: Exact plan revision containing the active claims.
    ///
    /// Branch and result identities for newly settled workers.
    ///
    /// # Returns
    /// Settled `(branch, result)` pairs in stable plan-node order.
    #[must_use]
    pub fn settlement_result_refs_since(&self, previous: &Plan) -> Vec<(EntityId, EntityId)> {
        if self.id != previous.id {
            return Vec::new();
        }
        previous
            .nodes
            .iter()
            .filter_map(|prior| {
                if prior.status != PlanNodeStatus::Running {
                    return None;
                }
                let claim = prior.claim.as_ref()?;
                let settled = self.nodes.iter().find(|node| node.id == prior.id)?;
                if settled.branch != Some(claim.branch) {
                    return None;
                }
                let result = match settled.status {
                    PlanNodeStatus::Completed => settled.result,
                    PlanNodeStatus::Failed => settled
                        .last_failure
                        .as_ref()
                        .filter(|failure| failure.kind == crate::PlanEvidenceKind::Failure)
                        .map(|failure| failure.reference.entity),
                    _ => None,
                }?;
                Some((claim.branch, result))
            })
            .collect()
    }

    /// Completes a running node with its committed durable branch result.
    ///
    /// This operation settles only the node claim. Plan-wide completion is
    /// evaluated separately against a complete session result snapshot.
    ///
    /// - `self`: Plan containing the node.
    /// - `node`: Running node to complete.
    /// - `result`: Durable result committed on the executing branch.
    ///
    /// Returns `ResultMismatch` if the durable result does not satisfy the
    /// node contract.
    /// # Parameters
    /// - `claim_id`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_node(
        &mut self,
        node: EntityId,
        claim_id: EntityId,
        result: &crate::BranchResult,
    ) -> Result<(), PlanError> {
        self.settle_workers(&[(node, claim_id, result.clone())], &[], &[])
    }

    /// Settles a concurrent worker batch as one durable plan revision.
    ///
    /// Successful results fulfill explicit dependency edges. Failed workers
    /// release their claims while leaving dependents blocked. The plan becomes
    /// complete in the same revision when every required result is present.
    ///
    /// - `self`: Plan containing the active worker claims.
    /// - `completed`: Exact `(node, claim, result)` completion records.
    /// - `failed`: Exact `(node, claim)` failed worker claims.
    /// - `available_results`: Durable branch results used for plan completion.
    ///
    /// Returns a plan error without partial mutation for stale claims, result
    /// mismatches, duplicate settlement entries, or revision exhaustion.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn settle_workers(
        &mut self,
        completed: &[(EntityId, EntityId, crate::BranchResult)],
        failed: &[(EntityId, EntityId, Option<crate::BranchResult>)],
        available_results: &[crate::BranchResult],
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if completed.is_empty() && failed.is_empty() {
            return Err(PlanError::InvalidTransition);
        }
        let mut proposed = self.clone();
        let mut settled_nodes = std::collections::BTreeSet::new();
        for (node_id, claim_id, result) in completed {
            if !settled_nodes.insert(*node_id) {
                return Err(PlanError::InvalidTransition);
            }
            let target = proposed
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
                .ok_or(PlanError::MissingNode)?;
            if target.status != PlanNodeStatus::Running
                || target.branch != Some(result.branch)
                || target.claim.as_ref().map(|claim| claim.id) != Some(*claim_id)
                || result.status != crate::BranchStatus::Completed
            {
                return Err(PlanError::InvalidTransition);
            }
            if target.expected_result.as_ref().is_some_and(|contract| {
                result.result_kind.as_deref() != Some(contract.kind.as_str())
                    || (contract.requires_handle
                        && result.result_handles.is_empty()
                        && result.references.is_empty())
            }) {
                return Err(PlanError::ResultMismatch);
            }
            target.result = Some(result.id);
            target.status = PlanNodeStatus::Completed;
            target.claim = None;
            for edge in proposed
                .dependencies
                .iter_mut()
                .filter(|edge| edge.prerequisite == *node_id)
            {
                edge.fulfilled_by = Some(result.id);
            }
        }
        for (node_id, claim_id, failure_result) in failed {
            if !settled_nodes.insert(*node_id) {
                return Err(PlanError::InvalidTransition);
            }
            let target = proposed
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
                .ok_or(PlanError::MissingNode)?;
            if target.status != PlanNodeStatus::Running
                || target.claim.as_ref().map(|claim| claim.id) != Some(*claim_id)
                || failure_result.as_ref().is_some_and(|result| {
                    Some(result.branch) != target.branch
                        || result.status == crate::BranchStatus::Completed
                })
            {
                return Err(PlanError::InvalidTransition);
            }
            target.status = PlanNodeStatus::Failed;
            target.claim = None;
            if let Some(result) = failure_result {
                target.last_failure = Some(crate::PlanEvidenceReference {
                    kind: crate::PlanEvidenceKind::Failure,
                    reference: crate::PlanSemanticReference {
                        entity: result.id,
                        revision: None,
                        snapshot: None,
                    },
                    attributes: std::collections::BTreeMap::from([
                        ("branch".to_owned(), result.branch.to_string()),
                        ("status".to_owned(), "failed".to_owned()),
                    ]),
                });
            }
        }
        proposed.refresh_readiness();
        if proposed.all_nodes_satisfied(available_results) {
            proposed.status = PlanStatus::Completed;
            proposed.coordinator_claim = None;
        }
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Fails an active worker claim without satisfying dependent nodes.
    ///
    /// - `self`: Plan containing the running node.
    /// - `node`: Stable plan-node identity.
    /// - `claim`: Exact active worker claim identity.
    ///
    /// Returns `InvalidTransition` when the node or claim is not active.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn fail_node(&mut self, node: EntityId, claim: EntityId) -> Result<(), PlanError> {
        self.settle_workers(&[], &[(node, claim, None)], &[])
    }
}
