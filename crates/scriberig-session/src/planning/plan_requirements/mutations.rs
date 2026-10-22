//! Applies atomic summary, evidence, gate, and decomposition mutations.

use scriberig_core::EntityId;

use super::super::{
    Plan, PlanAbandonReason, PlanError, PlanEvidenceAssociation, PlanEvidenceOwner, PlanGate,
    PlanGatePhase, PlanGateRequirement, PlanGateWaiver, PlanNode, PlanNodeParent, PlanNodeStatus,
    PlanNodeSummary,
};

impl Plan {
    /// Adds an independent typed gate to an existing plan node.
    ///
    /// # Parameters
    /// - `self`: Mutable plan receiving the gate.
    /// - `gate`: Stable gate identity and typed requirement.
    ///
    /// Unit after the gate and plan revision are committed.
    ///
    /// Returns a plan error without partial mutation when validation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns `MissingNode`, `DuplicateNode`, or a lifecycle/revision error.
    pub fn add_gate(&mut self, gate: PlanGate) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if self.gates.iter().any(|existing| existing.id == gate.id) {
            return Err(PlanError::DuplicateNode);
        }
        if self.semantic_entity_in_use(gate.id) {
            return Err(PlanError::DuplicateNode);
        }
        if !self.nodes.iter().any(|node| node.id == gate.owner) {
            return Err(PlanError::MissingNode);
        }
        if let PlanGateRequirement::RequiredChildCompleted { child } = &gate.requirement
            && !self.nodes.iter().any(|node| {
                node.id == *child && node.parent == Some(PlanNodeParent::Node(gate.owner))
            })
        {
            return Err(PlanError::MissingNode);
        }
        if gate.phase == PlanGatePhase::Entry
            && self
                .nodes
                .iter()
                .any(|node| node.id == gate.owner && node.status == PlanNodeStatus::Running)
        {
            return Err(PlanError::InvalidTransition);
        }
        let mut proposed = self.clone();
        proposed.gates.push(gate);
        proposed.refresh_readiness();
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Attaches an exact evidence association to a plan node or gate.
    ///
    /// # Parameters
    /// - `self`: Mutable plan receiving the evidence association.
    /// - `association`: Typed owner, role, exact reference, and safe note.
    ///
    /// Unit after evidence and plan revision are committed.
    ///
    /// Returns a plan error without partial mutation when validation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns `MissingNode`, `DuplicateNode`, or a lifecycle/revision error.
    pub fn attach_evidence(
        &mut self,
        association: PlanEvidenceAssociation,
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if self
            .evidence
            .iter()
            .any(|existing| existing.id == association.id)
            || self.semantic_entity_in_use(association.id)
        {
            return Err(PlanError::DuplicateNode);
        }
        let owner_exists = match association.owner {
            PlanEvidenceOwner::Node(node) => self.nodes.iter().any(|item| item.id == node),
            PlanEvidenceOwner::Gate(gate) => self.gates.iter().any(|item| item.id == gate),
        };
        if !owner_exists
            || association
                .note
                .as_ref()
                .is_some_and(|note| note.len() > 4096)
        {
            return Err(if owner_exists {
                PlanError::InvalidTransition
            } else {
                PlanError::MissingNode
            });
        }
        let mut proposed = self.clone();
        proposed.evidence.push(association);
        proposed.refresh_readiness();
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Replaces the current concise summary for a plan node.
    ///
    /// The current summary remains a separate semantic object and carries its
    /// exact source references. Replacing it does not change node purpose or
    /// final result.
    ///
    /// # Parameters
    /// - `self`: Mutable plan receiving the summary.
    /// - `summary`: Stable summary identity, node, text, and exact sources.
    ///
    /// Unit after the summary relation and plan revision are committed.
    ///
    /// Returns a plan error without partial mutation when validation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns `MissingNode`, `DuplicateNode`, or a lifecycle/revision error.
    pub fn update_summary(&mut self, summary: PlanNodeSummary) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if !self.nodes.iter().any(|node| node.id == summary.node) {
            return Err(PlanError::MissingNode);
        }
        if summary.text.trim().is_empty() || summary.text.len() > 4096 {
            return Err(PlanError::InvalidTransition);
        }
        if self
            .summaries
            .iter()
            .any(|known| known.id == summary.id && known.node != summary.node)
            || (self.semantic_entity_in_use(summary.id)
                && !self.summaries.iter().any(|known| known.id == summary.id))
        {
            return Err(PlanError::DuplicateNode);
        }
        let mut proposed = self.clone();
        proposed
            .summaries
            .retain(|known| known.node != summary.node);
        proposed.summaries.push(summary);
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Waives a gate while retaining its definition and authorization record.
    ///
    /// # Parameters
    /// - `self`: Mutable plan containing the gate.
    /// - `gate`: Stable gate identity to waive.
    /// - `waiver`: Safe reason and actor/operation provenance.
    ///
    /// Unit after the waiver and plan revision are committed.
    ///
    /// Returns a plan error without partial mutation when validation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns `MissingNode`, `InvalidTransition`, or a revision error.
    pub fn waive_gate(&mut self, gate: EntityId, waiver: PlanGateWaiver) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if waiver.reason.trim().is_empty() || waiver.reason.len() > 4096 {
            return Err(PlanError::InvalidTransition);
        }
        let mut proposed = self.clone();
        let Some(record) = proposed.gates.iter_mut().find(|record| record.id == gate) else {
            return Err(PlanError::MissingNode);
        };
        record.waiver = Some(waiver);
        proposed.refresh_readiness();
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Adds structural children and optional required-child completion gates.
    ///
    /// The operation creates no dependency edges. All children and required
    /// completion obligations are committed as one plan mutation revision.
    ///
    /// # Parameters
    /// - `self`: Mutable plan receiving the decomposition.
    /// - `parent`: Existing structural parent node.
    /// - `children`: New children with stable identities.
    /// - `required`: Whether every child must complete before the parent.
    ///
    /// Unit after all child and gate records are committed.
    ///
    /// Returns a plan error without partial mutation when validation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns `MissingNode`, `DuplicateNode`, or a lifecycle/revision error.
    pub fn decompose(
        &mut self,
        parent: EntityId,
        children: Vec<PlanNode>,
        required: bool,
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        if !self.nodes.iter().any(|node| node.id == parent) {
            return Err(PlanError::MissingNode);
        }
        let mut proposed = self.clone();
        for mut child in children {
            if proposed.semantic_entity_in_use(child.id) {
                return Err(PlanError::DuplicateNode);
            }
            if child.branch.is_some()
                || child.result.is_some()
                || child.claim.is_some()
                || child.claim_generation != 0
                || !matches!(
                    child.status,
                    PlanNodeStatus::Planned | PlanNodeStatus::Ready
                )
                || child.title.trim().is_empty()
                || child.purpose.trim().is_empty()
            {
                return Err(PlanError::InvalidTransition);
            }
            child.parent = Some(PlanNodeParent::Node(parent));
            child.status = PlanNodeStatus::Ready;
            proposed.nodes.push(child.clone());
            if required {
                proposed.gates.push(PlanGate {
                    id: EntityId::new(),
                    owner: parent,
                    phase: PlanGatePhase::Completion,
                    requirement: PlanGateRequirement::RequiredChildCompleted { child: child.id },
                    waiver: None,
                });
            }
        }
        if proposed.nodes.len() == self.nodes.len() {
            return Err(PlanError::InvalidTransition);
        }
        proposed.refresh_readiness();
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Marks one node abandoned while retaining a typed reason.
    ///
    /// # Parameters
    /// - `self`: Mutable plan containing the node.
    /// - `node`: Stable node identity to abandon.
    /// - `reason`: Typed reason retained in the node revision.
    ///
    /// Unit after abandonment and one plan revision are committed.
    ///
    /// Returns a plan error without partial mutation when validation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns `MissingNode`, `InvalidTransition`, or a revision error.
    pub fn abandon_node(
        &mut self,
        node: EntityId,
        reason: PlanAbandonReason,
    ) -> Result<(), PlanError> {
        self.ensure_mutable()?;
        let mut proposed = self.clone();
        let Some(target) = proposed.nodes.iter_mut().find(|item| item.id == node) else {
            return Err(PlanError::MissingNode);
        };
        if target.status == PlanNodeStatus::Running {
            return Err(PlanError::InvalidTransition);
        }
        target.status = PlanNodeStatus::Abandoned;
        target.abandon_reason = Some(reason);
        proposed.bump_revision()?;
        *self = proposed;
        Ok(())
    }

    /// Checks whether an identity is already owned by a plan semantic record.
    ///
    /// # Parameters
    /// - `self`: Canonical plan graph to inspect.
    /// - `identity`: Candidate entity identity for a new semantic record.
    ///
    /// `true` when the identity belongs to an existing plan/node/gate,
    /// evidence association, or summary.
    fn semantic_entity_in_use(&self, identity: EntityId) -> bool {
        identity == self.id
            || self.nodes.iter().any(|node| node.id == identity)
            || self.gates.iter().any(|gate| gate.id == identity)
            || self.evidence.iter().any(|item| item.id == identity)
            || self.summaries.iter().any(|item| item.id == identity)
    }
}
