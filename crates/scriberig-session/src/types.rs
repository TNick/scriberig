//! Defines session-level entities and cross-domain handles.
use crate::branch::Branch;
use crate::execution::SessionExecutionSelection;
use scriberig_core::{EntityId, Provenance};

/// Retains one sanitized legacy model choice for explicit operator recovery.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LegacyExecutionRecord {
    /// JSON pointer locating the original value in the prior session payload.
    pub path: String,
    /// Allowlisted historical selection fields; arbitrary properties are removed.
    pub value: serde_json::Value,
    /// Stable semantic owner, when this value came from a plan assignment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<LegacyExecutionTarget>,
}

/// Identifies the semantic plan location represented by a legacy assignment.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LegacyExecutionTarget {
    /// Stable plan identity encoded by the legacy payload.
    pub plan_id: EntityId,
    /// Stable plan node identity encoded by the legacy payload.
    pub node_id: EntityId,
    /// Historical plan revision, or `None` for the current assignment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    /// Whether the legacy value was a captured worker-claim assignment.
    #[serde(default)]
    pub claim: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Session {
    pub id: EntityId,
    pub branches: Vec<Branch>,
    pub goals: Vec<Goal>,
    pub tasks: Vec<Task>,
    /// Durable plans and their independent dependency graphs.
    #[serde(default)]
    pub plans: Vec<crate::Plan>,
    /// Durable selection used as the default for future model invocations.
    #[serde(default)]
    pub execution: SessionExecutionSelection,
}

impl Session {
    /// Adds a plan after validating its session and exact anchor.
    ///
    /// - `self`: Session receiving the plan.
    /// - `plan`: Complete durable plan value.
    ///
    /// Returns an error when the plan identity is duplicated, its owner differs,
    /// or its continuation anchor is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn add_plan(&mut self, plan: crate::Plan) -> Result<(), crate::SessionError> {
        if self.plans.iter().any(|existing| existing.id == plan.id) {
            return Err(crate::SessionError::DuplicatePlan);
        }
        if plan.session != Some(self.id) {
            return Err(crate::SessionError::PlanSessionMismatch);
        }
        let anchor = plan.anchor.ok_or(crate::SessionError::MissingPlanAnchor)?;
        let resolved = self.resolve_anchor(anchor)?;
        if resolved.branch != plan.root_branch {
            return Err(crate::SessionError::PlanAnchorMismatch);
        }
        if plan.nodes.iter().any(|node| {
            matches!(node.parent, Some(crate::PlanNodeParent::Branch(parent))
                if !self.branches.iter().any(|branch| branch.id == parent))
        }) {
            return Err(crate::SessionError::MissingParent);
        }
        self.plans.push(plan);
        Ok(())
    }

    /// Completes a plan node only after its result is committed on a branch.
    ///
    /// - `plan_id`: Stable plan identity.
    /// - `node_id`: Stable plan-node identity.
    /// - `result_id`: Durable branch-result identity already in this session.
    ///
    /// Returns `MissingNode` or `ResultMismatch` when the plan, node, or result
    /// is absent or does not match the node's executing branch.
    /// # Parameters
    /// - `claim_id`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_plan_node(
        &mut self,
        plan_id: EntityId,
        node_id: EntityId,
        claim_id: EntityId,
        result_id: EntityId,
    ) -> Result<(), crate::PlanError> {
        let result = self
            .branches
            .iter()
            .filter_map(|branch| branch.result.as_ref())
            .find(|result| result.id == result_id)
            .cloned()
            .ok_or(crate::PlanError::ResultMismatch)?;
        let plan = self
            .plans
            .iter_mut()
            .find(|plan| plan.id == plan_id)
            .ok_or(crate::PlanError::MissingNode)?;
        plan.complete_node(node_id, claim_id, &result)
    }

    /// Derives ready plan nodes from results in this exact session snapshot.
    ///
    /// - `plan_id`: Stable plan identity to evaluate.
    ///
    /// Stable identities of ready nodes, or an empty list for an unknown plan.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn ready_plan_nodes(&self, plan_id: EntityId) -> Vec<EntityId> {
        let Some(plan) = self.plans.iter().find(|plan| plan.id == plan_id) else {
            return Vec::new();
        };
        let results = self
            .branches
            .iter()
            .filter_map(|branch| branch.result.clone())
            .collect::<Vec<_>>();
        plan.ready_nodes_from_results(&results)
    }

    /// Resolves an exact replyable anchor against this session snapshot.
    ///
    /// - `anchor`: Stable continuation identity to validate.
    ///
    /// The branch and exact turn cutoff represented by the anchor.
    ///
    /// Returns an error for missing branches, turns, results, or non-replyable turns.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn resolve_anchor(
        &self,
        anchor: crate::ContinuationAnchor,
    ) -> Result<crate::ResolvedContinuationAnchor, crate::SessionError> {
        use crate::ContinuationAnchor as Anchor;

        let branch_id = match anchor {
            Anchor::BranchHead { branch }
            | Anchor::AssistantOutput { branch, .. }
            | Anchor::HumanInput { branch, .. }
            | Anchor::BranchResult { branch, .. } => branch,
        };
        let branch = self
            .branches
            .iter()
            .find(|item| item.id == branch_id)
            .ok_or(crate::SessionError::MissingAnchorBranch)?;
        let resolved = match anchor {
            Anchor::BranchHead { .. } => crate::ResolvedContinuationAnchor {
                branch: branch_id,
                turn_count: branch.turns.len(),
                result: None,
            },
            Anchor::AssistantOutput { turn, .. } => {
                let index = branch
                    .turns
                    .iter()
                    .position(|item| item.id == turn)
                    .ok_or(crate::SessionError::MissingAnchorTarget)?;
                if branch.turns[index].agent.is_none() {
                    return Err(crate::SessionError::NonReplyableAnchor);
                }
                crate::ResolvedContinuationAnchor {
                    branch: branch_id,
                    turn_count: index + 1,
                    result: None,
                }
            }
            Anchor::HumanInput { turn, .. } => {
                let index = branch
                    .turns
                    .iter()
                    .position(|item| item.id == turn)
                    .ok_or(crate::SessionError::MissingAnchorTarget)?;
                if branch.turns[index].human.is_none() {
                    return Err(crate::SessionError::NonReplyableAnchor);
                }
                crate::ResolvedContinuationAnchor {
                    branch: branch_id,
                    turn_count: index + 1,
                    result: None,
                }
            }
            Anchor::BranchResult { result, .. } => {
                if branch.result.as_ref().map(|item| item.id) != Some(result) {
                    return Err(crate::SessionError::MissingAnchorTarget);
                }
                crate::ResolvedContinuationAnchor {
                    branch: branch_id,
                    turn_count: branch.turns.len(),
                    result: Some(result),
                }
            }
        };
        Ok(resolved)
    }

    /// Creates a child branch rooted at a validated exact continuation point.
    ///
    /// - `id`: Stable identity for the new branch.
    /// - `anchor`: Existing durable reply point.
    /// - `title`: Human-readable branch title.
    /// - `purpose`: Focus for the continuation.
    /// - `origin`: Why the branch was created.
    ///
    /// Unit after the new branch is linked and retains its origin.
    ///
    /// Returns a session error if the anchor or branch creation is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn add_continuation_branch(
        &mut self,
        id: EntityId,
        anchor: crate::ContinuationAnchor,
        title: impl Into<String>,
        purpose: impl Into<String>,
        origin: crate::ContinuationOrigin,
    ) -> Result<(), crate::SessionError> {
        let resolved = self.resolve_anchor(anchor)?;
        self.add_child_branch(id, resolved.branch, title, purpose)?;
        let child = self
            .branches
            .iter_mut()
            .find(|branch| branch.id == id)
            .ok_or(crate::SessionError::DuplicateBranch)?;
        child.continuation = Some(crate::ContinuationProvenance { anchor, origin });
        Ok(())
    }

    /// Inserts a child branch and updates the parent adjacency list.
    ///
    /// - `self`: Session graph being updated.
    /// - `id`: Stable child branch identifier.
    /// - `parent`: Existing parent branch identifier.
    /// - `title`: Human-readable child title.
    /// - `purpose`: Focused work assigned to the child.
    ///
    /// Unit after the child is linked into the session graph.
    ///
    /// Returns `MissingParent` when the parent is absent and `DuplicateBranch`
    /// when the child identifier is already present.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn add_child_branch(
        &mut self,
        id: EntityId,
        parent: EntityId,
        title: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Result<(), crate::SessionError> {
        if self.branches.iter().any(|branch| branch.id == id) {
            return Err(crate::SessionError::DuplicateBranch);
        }
        let parent_branch = self
            .branches
            .iter_mut()
            .find(|branch| branch.id == parent)
            .ok_or(crate::SessionError::MissingParent)?;
        let child = Branch::new_child(id, parent, title, purpose)?;
        parent_branch.children.push(id);
        parent_branch.children.sort();
        self.branches.push(child);
        self.branches.sort_by_key(|branch| branch.id);
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Goal {
    pub id: EntityId,
    pub owner: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Task {
    pub id: EntityId,
    pub owner: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Observation {
    pub id: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Question {
    pub id: EntityId,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Decision {
    pub id: EntityId,
    pub branch: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct BranchResult {
    pub id: EntityId,
    pub branch: EntityId,
    pub status: crate::branch::BranchStatus,
    pub summary: Option<String>,
    pub result_kind: Option<String>,
    /// Exact project/candidate snapshots associated with this result.
    #[serde(default)]
    pub candidate_context: Option<BranchCandidateContext>,
    pub references: Vec<EntityId>,
    /// Stable non-entity handles produced by the branch.
    #[serde(default)]
    pub result_handles: Vec<String>,
    pub touched: Vec<EntityId>,
    pub produced_changes: Vec<EntityId>,
    pub validations: Vec<EntityId>,
    pub unresolved_questions: Vec<EntityId>,
}

/// Identifies the exact candidate context produced by semantic branch work.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct BranchCandidateContext {
    /// Semantic project containing the candidate.
    pub project: scriberig_core::ProjectId,
    /// Snapshot used as the candidate base.
    pub base_snapshot: scriberig_core::SnapshotId,
    /// Exact candidate snapshot produced by the branch.
    pub candidate_snapshot: scriberig_core::SnapshotId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Session-domain structured value.
pub struct Summary {
    pub id: EntityId,
    pub text: String,
    pub provenance: Provenance,
}
