//! Verifies atomic failed-worker settlement through SQLite.

use super::*;

/// Returns one deterministic worker execution failure.
struct FailedWorker;

impl PlanWorkerExecutor for FailedWorker {
    /// Fails the claimed work with a safe stable diagnostic.
    ///
    /// # Parameters
    /// - `_node`: Claimed plan node.
    /// - `_claim`: Exclusive branch claim.
    /// - `_execution`: Resolved upstream model assignment.
    ///
    /// # Returns
    /// This executor always returns an error for settlement coverage.
    ///
    /// # Errors
    /// Returns the deterministic worker failure.
    fn execute(
        &self,
        _node: &PlanNode,
        _claim: &scriberig_session::WorkerClaim,
        _execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<scriberig_session::BranchResult, String> {
        Err("provider unavailable".to_owned())
    }
}

/// Persists failed worker provenance without fulfilling dependencies.
#[test]
#[allow(clippy::too_many_lines)]
fn coordinator_atomically_settles_failed_worker_for_recovery() {
    let service = ApplicationService::new(Backend {
        store: std::sync::Arc::new(MemoryStore::new()),
    })
    .with_session_store(std::sync::Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open_in_memory()
            .expect("open durable session store"),
    ))
    .with_upstream_models(runtime_fallback_models());
    let session_id = EntityId::from_bytes([91; 16]);
    let root = EntityId::from_bytes([92; 16]);
    let plan_id = EntityId::from_bytes([93; 16]);
    let failing = EntityId::from_bytes([94; 16]);
    let dependent = EntityId::from_bytes([95; 16]);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("read grant"),
            CapabilityId::new("agent_plan").expect("plan grant"),
        ]),
        ..ApplicationContext::empty()
    };
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: root },
        "recover after provider failure",
    )
    .expect("plan");
    for (id, title) in [(failing, "fail"), (dependent, "blocked")] {
        plan.add_node(PlanNode {
            id,
            parent: Some(PlanNodeParent::Branch(root)),
            title: title.to_owned(),
            purpose: title.to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: PlanNodeStatus::Planned,
            branch: None,
            result: None,
            superseded_by: None,
            abandon_reason: None,
            last_failure: None,
            assignment: None,
            claim: None,
            claim_generation: 0,
        })
        .expect("add node");
    }
    plan.add_dependency(failing, dependent)
        .expect("add dependency");
    plan.transition(PlanStatus::Planned).expect("plan plan");
    let revision = plan.revision;
    let mut session = Session {
        id: session_id,
        branches: vec![Branch::new(root, "root").expect("root branch")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    session.add_plan(plan).expect("attach plan");
    SessionBackend::save_session(&service, &context, &session).expect("save session");
    let started = service
        .start_plan(
            &context,
            &scriberig_app::PlanLifecycleRequest {
                session: session_id,
                plan: plan_id,
                expected_revision: revision,
            },
        )
        .expect("start plan");
    let coordinator_claim = started
        .coordinator_claim
        .as_ref()
        .expect("coordinator claim")
        .id;

    let settled = PlanCoordinator::advance_once(
        &service,
        &context,
        session_id,
        plan_id,
        coordinator_claim,
        &FailedWorker,
        &runtime_fallback_models(),
        "fallback-model",
        1,
        &mut || EntityId::from_bytes([96; 16]),
    )
    .expect("settle failed worker");
    let durable = SessionBackend::read_session(
        &service,
        &context,
        &SessionRequest {
            session: session_id,
        },
    )
    .expect("reload durable state");
    let failed_node = durable.plans[0]
        .nodes
        .iter()
        .find(|node| node.id == failing)
        .expect("failed node");
    let failed_branch = durable
        .branches
        .iter()
        .find(|branch| Some(branch.id) == failed_node.branch)
        .expect("failed branch");
    assert_eq!(settled.workers.failed, vec![failing]);
    assert_eq!(failed_node.status, PlanNodeStatus::Failed);
    assert!(failed_node.claim.is_none());
    assert!(failed_node.last_failure.is_some());
    assert_eq!(
        failed_branch.result.as_ref().map(|result| result.status),
        Some(BranchStatus::Abandoned)
    );
    assert_eq!(durable.plans[0].dependencies[0].fulfilled_by, None);
    let branch_results = durable
        .branches
        .iter()
        .filter_map(|branch| branch.result.clone())
        .collect::<Vec<_>>();
    let mut retryable_plan = durable.plans[0].clone();
    retryable_plan
        .retry_failed_node(failing, &branch_results)
        .expect("failed plan node remains retryable");
    assert_eq!(retryable_plan.nodes[0].status, PlanNodeStatus::Ready);
}
