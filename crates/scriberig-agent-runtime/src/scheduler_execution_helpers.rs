//! Compiles, invokes, and settles claimed plan workers.

use scriberig_core::EntityId;
use scriberig_session::{BranchResult, Plan, PlanNode, WorkerClaim};

use crate::plan_context::PlanWorkerContext;
use crate::scheduler::{
    PlanWorkerExecutionResult, PlanWorkerExecutor,
    scheduler_helpers::{claimed_branch, failed_worker_result},
};

type ClaimedWorker = (
    PlanNode,
    WorkerClaim,
    scriberig_app::ResolvedPlanExecution,
    PlanWorkerContext,
);

/// Resolves each dispatched node through the upstream model catalog.
///
/// - `plan`: Durable plan snapshot being scheduled.
/// - `dispatch`: Stable IDs of selected ready nodes.
/// - `upstream_models`: Host-owned upstream model catalog.
/// - `selection`: Session-level defaults for unassigned nodes.
/// - `profile`: Optional agent profile used for profile-specific plan assignments.
/// - `runtime_fallback`: Runtime model used when no assignment is selected.
///
/// One resolved execution assignment per dispatched node, in dispatch order.
/// # Returns
/// The resolved worker assignments.
/// # Errors
/// Returns an error when a node or its assignment is unavailable.
pub(super) fn resolve_assignments(
    plan: &Plan,
    dispatch: &[EntityId],
    upstream_models: &[rpi_ai::Model],
    selection: &scriberig_session::SessionExecutionSelection,
    profile: Option<&scriberig_agent_core::AgentProfileDescriptor>,
    runtime_fallback: &str,
) -> Result<Vec<scriberig_app::ResolvedPlanExecution>, String> {
    dispatch
        .iter()
        .map(|node_id| {
            let node = plan
                .nodes
                .iter()
                .find(|node| node.id == *node_id)
                .ok_or_else(|| format!("dispatched plan node {node_id} is missing"))?;
            let resolved = match profile {
                Some(profile) => crate::profile::resolve_profile_plan_node_execution(
                    profile,
                    node,
                    scriberig_app::ExecutionSelectionSources {
                        invocation: None,
                        plan_node: None,
                        branch: None,
                        session: Some(selection),
                        project: None,
                        user_profile: None,
                    },
                    upstream_models,
                    runtime_fallback,
                ),
                None => scriberig_app::resolve_plan_node_execution(
                    node,
                    selection,
                    upstream_models,
                    runtime_fallback,
                ),
            };
            resolved.map_err(|error| format!("plan node assignment resolution failed: {error}"))
        })
        .collect()
}

/// Claims nodes, compiles exact contexts, and persists ownership before work.
///
/// - `plan`: Mutable plan projection receiving worker claims.
/// - `dispatch`: Stable IDs selected by the scheduler tick.
/// - `results`: Durable results used by dependency readiness checks.
/// - `resolved`: Assignments matching dispatch order.
/// - `next_id`: Allocates branch and claim identities.
/// - `compile_context`: Builds exact context for each claimed worker.
/// - `persist_claims`: Persists ownership before provider invocation.
///
/// Claimed worker snapshots with their exact assignments and contexts.
/// # Returns
/// The claims ready to execute.
/// # Errors
/// Returns an error when claiming, context compilation, or persistence fails.
pub(super) fn claim_workers(
    plan: &mut Plan,
    dispatch: &[EntityId],
    results: &[BranchResult],
    resolved: &[scriberig_app::ResolvedPlanExecution],
    mut next_id: impl FnMut() -> EntityId,
    compile_context: impl Fn(&Plan, &PlanNode, &WorkerClaim) -> Result<PlanWorkerContext, String>,
    persist_claims: impl FnOnce(&Plan) -> Result<(), String>,
) -> Result<Vec<ClaimedWorker>, String> {
    let mut claimed_plan = plan.clone();
    let claims = dispatch
        .iter()
        .map(|node_id| (*node_id, next_id(), next_id()))
        .collect::<Vec<_>>();
    claimed_plan
        .start_nodes(&claims, "plan-worker", results)
        .map_err(|error| format!("plan worker claim failed: {error:?}"))?;
    let mut claimed = Vec::with_capacity(dispatch.len());
    for (index, (node_id, _, _)) in claims.iter().enumerate() {
        let node = claimed_plan
            .nodes
            .iter()
            .find(|node| node.id == *node_id)
            .ok_or_else(|| "claimed plan node disappeared".to_owned())?
            .clone();
        let claim = node
            .claim
            .clone()
            .ok_or_else(|| "claimed plan node has no worker claim".to_owned())?;
        let context = compile_context(&claimed_plan, &node, &claim)?;
        claimed.push((node, claim, resolved[index].clone(), context));
    }
    if !claimed.is_empty() {
        persist_claims(&claimed_plan)?;
    }
    *plan = claimed_plan;
    Ok(claimed)
}

/// Runs each claimed worker on its own concurrent execution thread.
///
/// - `claimed`: Immutable claims and contexts captured before invocation.
/// - `executor`: Shared provider and runtime boundary.
///
/// Worker results paired with their node and claim identities.
/// # Returns
/// One execution result for every claim.
/// # Errors
/// Worker failures are returned alongside their node and claim identities.
pub(super) fn execute_workers(
    claimed: &[ClaimedWorker],
    executor: &impl PlanWorkerExecutor,
) -> Vec<(
    EntityId,
    EntityId,
    Result<PlanWorkerExecutionResult, String>,
)> {
    std::thread::scope(|scope| {
        let handles = claimed
            .iter()
            .map(|(node, claim, execution, context)| {
                scope.spawn(move || {
                    (
                        node.id,
                        claim.id,
                        executor.execute_with_context_and_requests(node, claim, execution, context),
                    )
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .enumerate()
            .map(|(index, handle)| {
                handle.join().unwrap_or_else(|_| {
                    let (node, claim, _, _) = &claimed[index];
                    (node.id, claim.id, Err("worker thread failed".to_owned()))
                })
            })
            .collect()
    })
}

/// Links execution outcomes to their claimed nodes and durable results.
///
/// - `plan`: Mutable plan receiving terminal worker state.
/// - `results`: Existing durable dependency results.
/// - `claimed`: Worker identities and exact branch snapshots.
/// - `executions`: Provider results returned by worker threads.
/// - `next_id`: Allocates stable failure result identities.
///
/// Completed and failed node IDs with branch results for persistence.
/// # Returns
/// The settled worker batch.
/// # Errors
/// Returns an error when the plan rejects the worker settlement.
pub(super) fn settle_workers(
    plan: &mut Plan,
    results: &[BranchResult],
    claimed: &[ClaimedWorker],
    executions: Vec<(
        EntityId,
        EntityId,
        Result<PlanWorkerExecutionResult, String>,
    )>,
    mut next_id: impl FnMut() -> EntityId,
) -> Result<super::WorkerBatchResult, String> {
    let mut batch = super::WorkerBatchResult {
        completed: Vec::new(),
        failed: Vec::new(),
        results: Vec::new(),
        model_request_evidence: Vec::new(),
        tool_evidence: Vec::new(),
    };
    let mut completed = Vec::new();
    let mut failed = Vec::new();
    for (node, claim, execution) in executions {
        let branch = claimed_branch(claimed, claim);
        match execution {
            Ok(execution)
                if execution.result.branch == branch
                    && execution.result.status == scriberig_session::BranchStatus::Completed =>
            {
                batch.completed.push(node);
                completed.push((node, claim, execution.result.clone()));
                batch.results.push(execution.result);
                batch
                    .model_request_evidence
                    .push(super::WorkerModelRequestEvidence {
                        branch,
                        requests: execution.model_requests,
                    });
                batch.tool_evidence.push(super::WorkerToolEvidence {
                    branch,
                    evidence: execution.tool_evidence,
                });
            }
            Ok(execution) if execution.result.branch == branch => {
                batch.failed.push(node);
                failed.push((node, claim, Some(execution.result.clone())));
                batch.results.push(execution.result);
                batch
                    .model_request_evidence
                    .push(super::WorkerModelRequestEvidence {
                        branch,
                        requests: execution.model_requests,
                    });
                batch.tool_evidence.push(super::WorkerToolEvidence {
                    branch,
                    evidence: execution.tool_evidence,
                });
            }
            Ok(_) => {
                batch.failed.push(node);
                let failure = failed_worker_result(
                    next_id(),
                    branch,
                    "worker returned a result for another branch",
                );
                failed.push((node, claim, Some(failure.clone())));
                batch.results.push(failure);
            }
            Err(error) => {
                batch.failed.push(node);
                let failure = failed_worker_result(next_id(), branch, &error);
                failed.push((node, claim, Some(failure.clone())));
                batch.results.push(failure);
            }
        }
    }
    let mut available_results = results.to_vec();
    available_results.extend(batch.results.iter().cloned());
    plan.settle_workers(&completed, &failed, &available_results)
        .map_err(|error| format!("plan worker settlement failed: {error:?}"))?;
    Ok(batch)
}

#[cfg(test)]
mod tests {
    //! Verifies scheduler assignment resolution includes semantic profile defaults.

    use super::resolve_assignments;
    use scriberig_agent_core::AgentProfileExecutionPreference;
    use scriberig_core::EntityId;
    use scriberig_session::{
        ContinuationAnchor, Plan, PlanNode, PlanNodeParent, PlanNodeStatus,
        SessionExecutionSelection,
    };

    /// Resolves an unassigned worker from its captured profile preference.
    #[test]
    fn worker_assignment_uses_profile_preference() {
        let session = EntityId::from_bytes([201; 16]);
        let root = EntityId::from_bytes([202; 16]);
        let node_id = EntityId::from_bytes([203; 16]);
        let mut plan = Plan::new(
            EntityId::from_bytes([204; 16]),
            session,
            ContinuationAnchor::BranchHead { branch: root },
            "profile worker selection",
        )
        .expect("plan");
        plan.nodes.push(PlanNode {
            id: node_id,
            parent: Some(PlanNodeParent::Branch(root)),
            title: "worker".to_owned(),
            purpose: "worker purpose".to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: PlanNodeStatus::Planned,
            branch: None,
            result: None,
            superseded_by: None,
            assignment: None,
            claim: None,
            claim_generation: 0,
            abandon_reason: None,
            last_failure: None,
        });
        let mut profile = crate::profile::bootstrap_default_agent_profile();
        profile.preferred_execution = Some(AgentProfileExecutionPreference {
            provider_id: "preferred-provider".to_owned(),
            model_id: "preferred-model".to_owned(),
            thinking_level: None,
        });
        let models = vec![rpi_ai::Model::new(
            "preferred-model",
            "Preferred model",
            rpi_ai::Api::AnthropicMessages,
            "preferred-provider",
            "https://example.test",
        )];

        let resolved = resolve_assignments(
            &plan,
            &[node_id],
            &models,
            &SessionExecutionSelection::default(),
            Some(&profile),
            "preferred-model",
        )
        .expect("worker selection resolves");

        assert_eq!(resolved[0].provider_id, "preferred-provider");
        assert_eq!(resolved[0].model_id, "preferred-model");
        assert!(!resolved[0].uses_runtime_fallback);
    }
}
