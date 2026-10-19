//! Implements one responsibility of durable plan lifecycle behavior.

use super::{
    CoordinatorClaim, EntityId, OperationId, Plan, PlanError, PlanNodeStatus, PlanStatus,
    WorkerClaim,
};

impl Plan {
    /// Assigns the plan's single auto-advancing coordinator branch and cursor.
    ///
    /// - `self`: Plan to update.
    /// - `branch`: Stable session branch authorized to coordinate.
    /// - `cursor`: Optional stable plan node currently being coordinated.
    ///
    /// Returns `MissingNode` when the optional cursor is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn set_coordinator(
        &mut self,
        branch: EntityId,
        cursor: Option<EntityId>,
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if cursor.is_some_and(|node| !self.nodes.iter().any(|entry| entry.id == node)) {
            return Err(PlanError::MissingNode);
        }
        self.coordinator = Some(branch);
        self.coordinator_cursor = cursor;
        self.bump_revision()?;
        Ok(())
    }

    /// Claims the single auto-advancing coordinator slot.
    ///
    /// - `self`: Running plan to claim.
    /// - `id`: Stable claim identity.
    /// - `owner`: Runtime process identity.
    ///
    /// Returns `InvalidTransition` when the plan is not running or already
    /// has a coordinator, and `RevisionExhausted` at generation overflow.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn claim_coordinator(
        &mut self,
        id: EntityId,
        owner: impl Into<String>,
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if self.status != PlanStatus::Running || self.coordinator_claim.is_some() {
            return Err(PlanError::InvalidTransition);
        }
        self.coordinator_generation = self
            .coordinator_generation
            .checked_add(1)
            .ok_or(PlanError::RevisionExhausted)?;
        self.coordinator_claim = Some(CoordinatorClaim {
            id,
            operation: None,
            owner: owner.into(),
            generation: self.coordinator_generation,
        });
        self.bump_revision()?;
        Ok(())
    }

    /// Starts or resumes auto-advancement with one atomic coordinator claim.
    ///
    /// The lifecycle transition and first claim share one plan revision so
    /// repositories can persist them with one compare-and-swap.
    ///
    /// - `self`: Plan to start or resume.
    /// - `id`: Stable coordinator claim identity.
    /// - `owner`: Runtime process identity.
    ///
    /// Returns `InvalidTransition`, `TerminalPlan`, or `RevisionExhausted` when
    /// the plan cannot be started or the claim generation cannot advance.
    /// # Parameters
    /// - `operation`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn start_with_coordinator_claim(
        &mut self,
        id: EntityId,
        owner: impl Into<String>,
        operation: Option<OperationId>,
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if !matches!(self.status, PlanStatus::Planned | PlanStatus::Paused)
            || self.coordinator_claim.is_some()
        {
            return Err(PlanError::InvalidTransition);
        }
        self.coordinator_generation = self
            .coordinator_generation
            .checked_add(1)
            .ok_or(PlanError::RevisionExhausted)?;
        self.coordinator_claim = Some(CoordinatorClaim {
            id,
            operation,
            owner: owner.into(),
            generation: self.coordinator_generation,
        });
        self.status = PlanStatus::Running;
        self.bump_revision()?;
        Ok(())
    }

    /// Releases a coordinator claim only when the caller owns its exact identity.
    ///
    /// - `self`: Plan holding the active claim.
    /// - `claim`: Claim identity to release.
    ///
    /// Returns `InvalidTransition` when the claim is absent or stale.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn release_coordinator(&mut self, claim: EntityId) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if self.coordinator_claim.as_ref().map(|current| current.id) != Some(claim) {
            return Err(PlanError::InvalidTransition);
        }
        self.coordinator_claim = None;
        self.bump_revision()?;
        Ok(())
    }

    /// Resolves claims whose runtime owners are no longer active.
    ///
    /// Abandoned worker executions become failed so their partial branch is
    /// never silently re-executed. Dependents remain blocked until explicit
    /// replan or retry policy changes the durable node state.
    ///
    /// - `self`: Durable plan reconstructed after runtime startup.
    /// - `owner_is_active`: Runtime liveness check for a claim owner.
    ///
    /// Number of coordinator and worker claims released as stale.
    ///
    /// Returns `RevisionExhausted` if the durable recovery revision cannot advance.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn recover_stale_claims(
        &mut self,
        mut owner_is_active: impl FnMut(&str) -> bool,
    ) -> Result<usize, PlanError> {
        self.ensure_mutable()?;
        let mut recovered = 0;
        if self
            .coordinator_claim
            .as_ref()
            .is_some_and(|claim| !owner_is_active(&claim.owner))
        {
            self.coordinator_claim = None;
            recovered += 1;
        }
        for node in &mut self.nodes {
            if node.status == PlanNodeStatus::Running
                && node
                    .claim
                    .as_ref()
                    .is_some_and(|claim| !owner_is_active(&claim.owner))
            {
                node.claim = None;
                node.status = PlanNodeStatus::Failed;
                recovered += 1;
            }
        }
        if recovered > 0 {
            self.bump_revision()?;
        }
        Ok(recovered)
    }

    /// Starts a ready node on one active branch.
    ///
    /// - `self`: Plan containing the node.
    /// - `node`: Ready node to start.
    /// - `branch`: Stable executing branch identity.
    ///
    /// Returns `MissingNode` or `InvalidTransition` when the node is not ready.
    /// # Parameters
    /// - `claim_id`: Input for this operation.
    /// - `owner`: Input for this operation.
    /// - `results`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn start_node(
        &mut self,
        node: EntityId,
        branch: EntityId,
        claim_id: EntityId,
        owner: impl Into<String>,
        results: &[crate::BranchResult],
    ) -> Result<(), PlanError> {
        self.start_nodes(&[(node, branch, claim_id)], owner, results)
    }

    /// Claims a ready worker batch as one durable plan revision.
    ///
    /// - `self`: Plan containing the ready nodes.
    /// - `workers`: Exact `(node, branch, claim)` identities to claim.
    /// - `owner`: Runtime owner identity shared by this dispatch batch.
    /// - `results`: Durable results used to validate dependency readiness.
    ///
    /// Returns a plan error without partial mutation when a node, branch, claim,
    /// dependency, or revision is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn start_nodes(
        &mut self,
        workers: &[(EntityId, EntityId, EntityId)],
        owner: impl Into<String>,
        results: &[crate::BranchResult],
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if self.status != PlanStatus::Running
            || self.coordinator_claim.is_none()
            || workers.is_empty()
        {
            return Err(PlanError::InvalidTransition);
        }
        let ready = self.ready_nodes_from_results(results);
        let owner = owner.into();
        let mut proposed = self.clone();
        let mut seen_nodes = std::collections::BTreeSet::new();
        let mut seen_branches = std::collections::BTreeSet::new();
        let mut seen_claims = std::collections::BTreeSet::new();
        for (node_id, branch_id, claim_id) in workers {
            if !seen_nodes.insert(*node_id)
                || !seen_branches.insert(*branch_id)
                || !seen_claims.insert(*claim_id)
                || proposed.nodes.iter().any(|node| {
                    node.branch == Some(*branch_id)
                        || node
                            .claim
                            .as_ref()
                            .is_some_and(|claim| claim.id == *claim_id)
                })
            {
                return Err(PlanError::InvalidTransition);
            }
            let target = proposed
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
                .ok_or(PlanError::MissingNode)?;
            if target.branch.is_some()
                || !ready.contains(node_id)
                || !matches!(
                    target.status,
                    PlanNodeStatus::Planned | PlanNodeStatus::Ready
                )
            {
                return Err(PlanError::InvalidTransition);
            }
            target.claim_generation = target
                .claim_generation
                .checked_add(1)
                .ok_or(PlanError::RevisionExhausted)?;
            target.status = PlanNodeStatus::Running;
            target.branch = Some(*branch_id);
            target.claim = Some(WorkerClaim {
                id: *claim_id,
                node: *node_id,
                branch: *branch_id,
                owner: owner.clone(),
                generation: target.claim_generation,
                assignment: target.assignment.clone(),
            });
        }
        proposed.status = PlanStatus::Running;
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }
}
