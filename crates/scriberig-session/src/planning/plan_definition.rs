//! Implements one responsibility of durable plan lifecycle behavior.

use super::{
    EntityId, ModelAssignment, Plan, PlanDependency, PlanError, PlanNode, PlanNodeParent,
    PlanNodeStatus, PlanStatus,
};

impl Plan {
    /// Creates a plan bound to an exact session continuation anchor.
    ///
    /// # Parameters
    /// - `id`: Stable plan identity.
    /// - `session`: Owning session identity.
    /// - `anchor`: Exact location where the plan begins.
    /// - `goal`: Durable objective for the plan.
    ///
    /// # Returns
    /// A draft plan with no nodes or coordinator claim.
    ///
    /// # Errors
    /// Returns `InvalidTransition` when the goal is empty.
    pub fn new(
        id: EntityId,
        session: EntityId,
        anchor: crate::ContinuationAnchor,
        goal: impl Into<String>,
    ) -> Result<Self, PlanError> {
        let root_branch = match anchor {
            crate::ContinuationAnchor::BranchHead { branch }
            | crate::ContinuationAnchor::AssistantOutput { branch, .. }
            | crate::ContinuationAnchor::HumanInput { branch, .. }
            | crate::ContinuationAnchor::BranchResult { branch, .. } => branch,
        };
        let goal = goal.into();
        if goal.trim().is_empty() {
            return Err(PlanError::InvalidTransition);
        }
        let mut plan = Self {
            id,
            session: Some(session),
            anchor: Some(anchor),
            goal,
            revision: 0,
            status: PlanStatus::Draft,
            root_branch,
            nodes: Vec::new(),
            dependencies: Vec::new(),
            coordinator: None,
            coordinator_cursor: None,
            coordinator_claim: None,
            coordinator_generation: 0,
            root_nodes: Vec::new(),
            history: Vec::new(),
        };
        plan.record_revision();
        Ok(plan)
    }

    /// Adds a planned node under an existing branch or plan node.
    ///
    /// - `self`: Plan receiving the new node.
    /// - `node`: New durable plan node.
    ///
    /// Returns `DuplicateNode` if the identity is already present.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn add_node(&mut self, node: PlanNode) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if self.nodes.iter().any(|existing| existing.id == node.id) {
            return Err(PlanError::DuplicateNode);
        }
        if let Some(PlanNodeParent::Node(parent)) = node.parent {
            if !self.nodes.iter().any(|existing| existing.id == parent) {
                return Err(PlanError::MissingNode);
            }
        }
        let node_id = node.id;
        let is_root = matches!(node.parent, Some(PlanNodeParent::Branch(_)) | None);
        self.nodes.push(node);
        if is_root {
            self.root_nodes.push(node_id);
        }
        self.bump_revision()?;
        self.refresh_readiness();
        Ok(())
    }

    /// Adds one explicitly discovered next step to a running plan.
    ///
    /// This is ordinary plan evolution, not replan replacement. It preserves
    /// existing node identities and advances the plan mutation revision once.
    ///
    /// # Parameters
    /// - `self`: Running plan receiving the new work.
    /// - `node`: New node with an already allocated stable identity.
    ///
    /// # Returns
    /// Unit after the next step is added.
    ///
    /// # Errors
    /// Returns `InvalidTransition` when the plan is not running, or propagates
    /// node validation and revision errors from `add_node`.
    pub fn add_next_step(&mut self, node: PlanNode) -> Result<(), PlanError> {
        if self.status != PlanStatus::Running {
            return Err(PlanError::InvalidTransition);
        }
        self.add_node(node)
    }

    /// Adds an edge after checking that it will not create a cycle.
    ///
    /// - `self`: Plan whose dependency graph is updated.
    /// - `prerequisite`: Node whose result must exist first.
    /// - `dependent`: Node blocked by the prerequisite.
    ///
    /// Returns `MissingNode` for an unknown node or `DependencyCycle` when the
    /// edge would make the dependency graph cyclic.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn add_dependency(
        &mut self,
        prerequisite: EntityId,
        dependent: EntityId,
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if prerequisite == dependent {
            return Err(PlanError::SelfDependency);
        }
        if !self.nodes.iter().any(|node| node.id == prerequisite)
            || !self.nodes.iter().any(|node| node.id == dependent)
        {
            return Err(PlanError::MissingNode);
        }
        let prerequisite_node = self
            .nodes
            .iter()
            .find(|node| node.id == prerequisite)
            .ok_or(PlanError::MissingNode)?;
        let dependent_node = self
            .nodes
            .iter()
            .find(|node| node.id == dependent)
            .ok_or(PlanError::MissingNode)?;
        if dependent_node.status == PlanNodeStatus::Completed
            || (prerequisite_node.status == PlanNodeStatus::Completed
                && prerequisite_node.result.is_none())
        {
            return Err(PlanError::InvalidTransition);
        }
        if self
            .dependencies
            .iter()
            .any(|edge| edge.prerequisite == prerequisite && edge.dependent == dependent)
        {
            return Ok(());
        }
        if self.reaches(dependent, prerequisite) {
            return Err(PlanError::DependencyCycle);
        }
        self.dependencies.push(PlanDependency {
            prerequisite,
            dependent,
            fulfilled_by: prerequisite_node.result,
        });
        self.dependencies
            .sort_by_key(|edge| (edge.prerequisite, edge.dependent));
        self.bump_revision()?;
        self.refresh_readiness();
        Ok(())
    }

    /// Applies one validated replan as a single durable revision.
    ///
    /// Superseded unfinished work is marked abandoned and linked to its
    /// replacement. Completed nodes and their exact results remain intact.
    /// Existing dependency edges that touch superseded nodes leave the current
    /// projection while remaining inspectable in plan history.
    ///
    /// - `self`: Paused or failed plan being revised.
    /// - `additions`: New structured nodes with already allocated identities.
    /// - `dependencies`: Explicit prerequisite/dependent identities to add.
    /// - `superseded`: Old-node and replacement-node identity pairs.
    ///
    /// Returns `MissingNode`, `InvalidTransition`, `DependencyCycle`, or
    /// `RevisionExhausted` when the proposal is inconsistent.
    /// # Parameters
    /// - `removed_dependencies`: Input for this operation.
    /// - `assignment_changes`: Input for this operation.
    /// - `preserved_terminal_nodes`: Input for this operation.
    /// - `reason`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn replan(
        &mut self,
        additions: Vec<PlanNode>,
        dependencies: Vec<(EntityId, EntityId)>,
        removed_dependencies: &[(EntityId, EntityId)],
        superseded: &[(EntityId, EntityId)],
        assignment_changes: &[(EntityId, Option<ModelAssignment>)],
        preserved_terminal_nodes: &[EntityId],
        reason: &str,
    ) -> Result<(), PlanError> {
        if !matches!(self.status, PlanStatus::Paused | PlanStatus::Failed)
            || reason.trim().is_empty()
        {
            return Err(PlanError::InvalidTransition);
        }
        let mut expected_preserved = self
            .nodes
            .iter()
            .filter(|node| node.status == PlanNodeStatus::Completed)
            .map(|node| node.id)
            .collect::<Vec<_>>();
        let mut actual_preserved = preserved_terminal_nodes.to_vec();
        expected_preserved.sort();
        actual_preserved.sort();
        if expected_preserved != actual_preserved {
            return Err(PlanError::InvalidTransition);
        }
        let addition_ids = additions.iter().map(|node| node.id).collect::<Vec<_>>();
        let mut proposed = self.clone();
        proposed.status = PlanStatus::Paused;
        proposed.coordinator_claim = None;
        for (old_id, replacement_id) in superseded {
            if !addition_ids.contains(replacement_id) {
                return Err(PlanError::MissingNode);
            }
            let old = proposed
                .nodes
                .iter_mut()
                .find(|node| node.id == *old_id)
                .ok_or(PlanError::MissingNode)?;
            if matches!(
                old.status,
                PlanNodeStatus::Completed | PlanNodeStatus::Running
            ) || old.superseded_by.is_some()
            {
                return Err(PlanError::InvalidTransition);
            }
            old.status = PlanNodeStatus::Abandoned;
            old.superseded_by = Some(*replacement_id);
        }
        proposed.dependencies.retain(|edge| {
            !superseded
                .iter()
                .any(|(old, _)| edge.prerequisite == *old || edge.dependent == *old)
        });
        for (prerequisite, dependent) in removed_dependencies {
            let index = proposed
                .dependencies
                .iter()
                .position(|edge| edge.prerequisite == *prerequisite && edge.dependent == *dependent)
                .ok_or(PlanError::MissingNode)?;
            if proposed
                .nodes
                .iter()
                .any(|node| node.id == *dependent && node.status == PlanNodeStatus::Completed)
            {
                return Err(PlanError::InvalidTransition);
            }
            proposed.dependencies.remove(index);
        }
        for node in additions {
            proposed.add_node(node)?;
        }
        for (prerequisite, dependent) in dependencies {
            proposed.add_dependency(prerequisite, dependent)?;
        }
        for (node, assignment) in assignment_changes {
            proposed.set_assignment(*node, assignment.clone())?;
        }
        proposed.revision = self.revision;
        proposed.history.clone_from(&self.history);
        proposed.bump_revision()?;
        if let Some(revision) = proposed.history.last_mut() {
            revision.reason = Some(reason.to_owned());
        }
        *self = proposed;
        Ok(())
    }

    /// Adds isolated sibling attempts and a comparison node atomically.
    ///
    /// The comparison node is dependency-blocked until every attempt publishes
    /// its own durable result.
    ///
    /// - `self`: Mutable plan receiving the attempt set.
    /// - `attempts`: Two or more structurally sibling attempt nodes.
    /// - `comparison`: Node that will compare all attempt results.
    ///
    /// Returns `InvalidTransition`, `DependencyCycle`, or a node validation
    /// error without changing the original plan.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn add_alternative_attempts(
        &mut self,
        attempts: Vec<PlanNode>,
        comparison: PlanNode,
    ) -> Result<(), PlanError> {
        if attempts.len() < 2
            || attempts
                .iter()
                .any(|attempt| attempt.parent != attempts[0].parent)
        {
            return Err(PlanError::InvalidTransition);
        }
        let mut proposed = self.clone();
        let original_revision = self.revision;
        let original_history = self.history.clone();
        let comparison_id = comparison.id;
        let attempt_ids = attempts
            .iter()
            .map(|attempt| attempt.id)
            .collect::<Vec<_>>();
        for attempt in attempts {
            proposed.add_node(attempt)?;
        }
        proposed.add_node(comparison)?;
        for attempt in attempt_ids {
            proposed.add_dependency(attempt, comparison_id)?;
        }
        proposed.revision = original_revision;
        proposed.history = original_history;
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }
}
