//! Compiles exact isolated context for one durable plan worker.

use crate::context::{
    BranchExecutionContext, BranchRole, CompactResultContext, CompiledContext, ContextActivity,
    ContextCompiler, ContextPolicy,
};
use scriberig_core::EntityId;
use scriberig_session::{
    BranchResult, ContinuationAnchor, Plan, PlanNode, PlanNodeParent, Session, WorkerClaim,
};

/// Holds selected durable inputs and their provider-ready compiled projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanWorkerContext {
    /// Structured inputs selected from the exact plan/session location.
    pub inputs: BranchExecutionContext,
    /// Bounded provider-neutral context compiled from those inputs.
    pub compiled: CompiledContext,
}

impl PlanWorkerContext {
    /// Creates a scope-only context when no durable session snapshot is supplied.
    ///
    /// This fallback is used by scheduler unit tests and low-level callers;
    /// application-backed coordination should use `from_plan`.
    ///
    /// - `plan`: Plan containing the worker.
    /// - `node`: Selected plan node.
    /// - `claim`: Exact worker claim.
    ///
    /// Compiled purpose and semantic scope without inherited conversation.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn minimal(plan: &Plan, node: &PlanNode, claim: &WorkerClaim) -> Self {
        let inputs = BranchExecutionContext::new(
            plan.session
                .map_or_else(|| "unknown".to_owned(), |id| id.to_string()),
            claim.branch.to_string(),
            worker_purpose(node),
            node.purpose.clone(),
        )
        .with_branch_role(BranchRole::NonRoot {
            parent: worker_parent_branch(plan, node).to_string(),
        });
        let compiled = ContextCompiler::new(ContextPolicy::default()).compile(&inputs);
        Self { inputs, compiled }
    }

    /// Compiles one worker context from its durable session, plan, and claim.
    ///
    /// Only the plan anchor, structural ancestor results, fulfilled dependency
    /// results, node semantic scope, and the claimed branch's local visible
    /// history are selected. Other branch transcripts are not added.
    ///
    /// - `session`: Exact durable session snapshot before provider execution.
    /// - `plan`: Exact durable plan revision containing the worker node.
    /// - `node`: Selected plan node.
    /// - `claim`: Exact branch and worker claim being executed.
    ///
    /// Selected structured inputs and bounded compiled model context.
    ///
    /// Returns an error when the plan anchor, node, ancestor result, or fulfilled
    /// dependency result cannot be resolved exactly.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn from_plan(
        session: &Session,
        plan: &Plan,
        node: &PlanNode,
        claim: &WorkerClaim,
    ) -> Result<Self, String> {
        Self::from_plan_with_profile(session, plan, node, claim, None)
    }

    /// Compiles one worker context against the exact supplied profile revision.
    ///
    /// - `session`: Exact durable session snapshot before provider execution.
    /// - `plan`: Exact durable plan revision containing the worker node.
    /// - `node`: Selected plan node.
    /// - `claim`: Exact branch and worker claim being executed.
    /// - `profile`: Exact semantic profile selected for this worker.
    ///
    /// Selected structured inputs and bounded compiled model context.
    ///
    /// # Returns
    /// The compiled context bound to the supplied profile revision.
    ///
    /// # Errors
    /// Returns an error when exact plan, branch, ancestor, or dependency state
    /// cannot be resolved.
    pub fn from_plan_with_profile(
        session: &Session,
        plan: &Plan,
        node: &PlanNode,
        claim: &WorkerClaim,
        profile: Option<&scriberig_agent_core::AgentProfileDescriptor>,
    ) -> Result<Self, String> {
        if !plan.nodes.iter().any(|candidate| candidate.id == node.id)
            || claim.node != node.id
            || claim.branch == EntityId::from_bytes([0; 16])
        {
            return Err("plan worker identity mismatch".to_owned());
        }
        let anchor = plan.anchor.unwrap_or(ContinuationAnchor::BranchHead {
            branch: plan.root_branch,
        });
        let purpose = worker_purpose(node);
        let mut inputs = BranchExecutionContext::from_session_anchor(
            session,
            claim.branch,
            anchor,
            purpose,
            node.purpose.clone(),
        )
        .map_err(|error| format!("plan worker continuation anchor invalid: {error}"))?;
        let worker_branch = session
            .branches
            .iter()
            .find(|branch| branch.id == claim.branch)
            .ok_or_else(|| "plan worker branch is missing from session".to_owned())?;
        let branch_role =
            worker_branch
                .parent
                .map_or(BranchRole::Root, |parent| BranchRole::NonRoot {
                    parent: parent.to_string(),
                });
        inputs = inputs.with_branch_role(branch_role);

        for result in structural_ancestor_results(session, plan, node)? {
            inputs = inputs.with_ancestor_result(result);
        }
        for result in dependency_results(session, plan, node)? {
            inputs = inputs.with_dependency_result(result);
        }
        add_local_branch_history(&mut inputs, session, claim.branch);

        let context_compiler = ContextCompiler::new(ContextPolicy::default());
        let compiled = match profile {
            Some(profile) => context_compiler
                .with_profile(profile.clone())
                .compile(&inputs),
            None => context_compiler.compile(&inputs),
        };
        Ok(Self { inputs, compiled })
    }
}

/// Resolves the parent branch assigned to a plan worker by coordinator policy.
///
/// `from_plan` prefers the persisted branch parent's identity. This helper is
/// for sessionless fallback work and matches the coordinator's creation rule.
///
/// - `plan`: Durable plan selecting the worker.
/// - `node`: Plan node assigned to the worker branch.
///
/// The parent branch identity, falling back to the plan root when the node
/// parent is absent or its parent node has no assigned branch.
/// # Returns
/// The resolved parent branch identity.
pub(crate) fn worker_parent_branch(plan: &Plan, node: &PlanNode) -> EntityId {
    match node.parent {
        Some(PlanNodeParent::Branch(branch)) => branch,
        Some(PlanNodeParent::Node(parent_node)) => plan
            .nodes
            .iter()
            .find(|candidate| candidate.id == parent_node)
            .and_then(|candidate| candidate.branch)
            .unwrap_or(plan.root_branch),
        None => plan.root_branch,
    }
}

/// Adds the node's exact semantic handles to the required worker purpose.
/// # Parameters
/// - `node`: Input for this operation.
/// # Returns
/// The operation result.
fn worker_purpose(node: &PlanNode) -> String {
    if node.semantic_scope.is_empty() {
        return node.purpose.clone();
    }
    format!(
        "{}\nExact semantic scope: {}",
        node.purpose,
        node.semantic_scope.join(", ")
    )
}

/// Selects completed results from structural plan ancestors only.
/// # Parameters
/// - `session`: Input for this operation.
/// - `plan`: Input for this operation.
/// - `node`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn structural_ancestor_results(
    session: &Session,
    plan: &Plan,
    node: &PlanNode,
) -> Result<Vec<CompactResultContext>, String> {
    let mut ancestors = Vec::new();
    let mut parent = node.parent;
    while let Some(PlanNodeParent::Node(parent_id)) = parent {
        let ancestor = plan
            .nodes
            .iter()
            .find(|candidate| candidate.id == parent_id)
            .ok_or_else(|| "plan structural ancestor is missing".to_owned())?;
        if let Some(result_id) = ancestor.result {
            ancestors.push(compact_result(session, result_id)?);
        }
        parent = ancestor.parent;
    }
    if let Some(PlanNodeParent::Branch(branch_id)) = parent
        && !matches!(
            plan.anchor,
            Some(ContinuationAnchor::BranchResult { branch, .. }) if branch == branch_id
        )
        && let Some(result) = session
            .branches
            .iter()
            .find(|branch| branch.id == branch_id)
            .and_then(|branch| branch.result.as_ref())
    {
        ancestors.push(compact_branch_result(result));
    }
    ancestors.reverse();
    Ok(ancestors)
}

/// Selects exact fulfilled results on the node's incoming dependency edges.
/// # Parameters
/// - `session`: Input for this operation.
/// - `plan`: Input for this operation.
/// - `node`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn dependency_results(
    session: &Session,
    plan: &Plan,
    node: &PlanNode,
) -> Result<Vec<CompactResultContext>, String> {
    plan.dependencies
        .iter()
        .filter(|edge| edge.dependent == node.id)
        .map(|edge| {
            let result_id = edge
                .fulfilled_by
                .ok_or_else(|| "plan dependency has no fulfilled result".to_owned())?;
            let prerequisite = plan
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.prerequisite)
                .ok_or_else(|| "plan dependency prerequisite is missing".to_owned())?;
            if prerequisite.status != scriberig_session::PlanNodeStatus::Completed
                || prerequisite.result != Some(result_id)
            {
                return Err("plan dependency result does not match its prerequisite".to_owned());
            }
            compact_result(session, result_id)
        })
        .collect()
}

/// Resolves an exact durable branch result to its compact context projection.
/// # Parameters
/// - `session`: Input for this operation.
/// - `result_id`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn compact_result(session: &Session, result_id: EntityId) -> Result<CompactResultContext, String> {
    let result = session
        .branches
        .iter()
        .filter_map(|branch| branch.result.as_ref())
        .find(|result| result.id == result_id)
        .ok_or_else(|| "selected durable branch result is missing".to_owned())?;
    Ok(compact_branch_result(result))
}

/// Converts a durable result into summary and semantic handles only.
/// # Parameters
/// - `result`: Input for this operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn compact_branch_result(result: &BranchResult) -> CompactResultContext {
    CompactResultContext {
        handle: format!("branch-result:{}", result.id),
        revision: result
            .candidate_context
            .map(|candidate| candidate.candidate_snapshot.to_string()),
        summary: result.summary.clone().unwrap_or_default(),
        semantic_handles: result.result_handles.clone(),
    }
}

/// Adds only the claimed branch's existing visible local conversation history.
/// # Parameters
/// - `inputs`: Input for this operation.
/// - `session`: Input for this operation.
/// - `branch_id`: Input for this operation.
fn add_local_branch_history(
    inputs: &mut BranchExecutionContext,
    session: &Session,
    branch_id: EntityId,
) {
    if let Some(branch) = session
        .branches
        .iter()
        .find(|branch| branch.id == branch_id)
    {
        for turn in &branch.turns {
            if let Some(human) = &turn.human {
                inputs.activities.push(
                    ContextActivity::new(
                        turn.ordinal.saturating_mul(2),
                        "human_input",
                        human.text.clone(),
                    )
                    .with_handle(format!("turn:{}:human", turn.id)),
                );
            }
            if let Some(agent) = &turn.agent {
                inputs.activities.push(
                    ContextActivity::new(
                        turn.ordinal.saturating_mul(2).saturating_add(1),
                        "assistant_output",
                        agent.text.clone(),
                    )
                    .with_handle(format!("turn:{}:assistant", turn.id)),
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "plan_context_tests.rs"]
mod tests;
