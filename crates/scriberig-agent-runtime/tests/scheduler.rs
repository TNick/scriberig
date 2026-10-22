//! Verifies pure durable-plan scheduling decisions.

use rpi_ai::{Api, Model};
use scriberig_agent_runtime::scheduler::{PlanScheduler, PlanWorkerExecutor};
use scriberig_core::EntityId;
#[path = "scheduler/worker.rs"]
mod worker;

use scriberig_session::{
    BranchResult, BranchStatus, ModelAssignment, Plan, PlanNode, PlanNodeParent, PlanNodeStatus,
    PlanStatus, WorkerClaim,
};

/// Builds the upstream catalog used by scheduler assignment tests.
///
/// The provider models used for plan assignment resolution.
/// # Returns
/// The operation result.
fn worker_models() -> Vec<Model> {
    vec![
        Model::new(
            "wire-a",
            "Model A",
            Api::AnthropicMessages,
            "provider",
            "https://example.test",
        ),
        Model::new(
            "wire-b",
            "Model B",
            Api::AnthropicMessages,
            "provider",
            "https://example.test",
        ),
        Model::new(
            "fallback-model",
            "Fallback Model",
            Api::AnthropicMessages,
            "provider",
            "https://example.test",
        ),
    ]
}

/// Builds a running plan with one active worker and one blocked dependent.
/// # Returns
/// The operation result.
fn scheduler_plan() -> (Plan, EntityId, EntityId, EntityId) {
    let session = EntityId::from_bytes([221; 16]);
    let root = EntityId::from_bytes([222; 16]);
    let plan_id = EntityId::from_bytes([223; 16]);
    let producer = EntityId::from_bytes([224; 16]);
    let ready = EntityId::from_bytes([225; 16]);
    let blocked = EntityId::from_bytes([226; 16]);
    let mut plan = Plan::new(
        plan_id,
        session,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root },
        "schedule test",
    )
    .expect("plan");
    for (id, title) in [
        (producer, "producer"),
        (ready, "ready"),
        (blocked, "blocked"),
    ] {
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
            assignment: None,
            claim: None,
            claim_generation: 0,
        })
        .expect("add node");
    }
    plan.add_dependency(producer, blocked)
        .expect("block dependent");
    plan.transition(PlanStatus::Planned).expect("plan plan");
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([227; 16]), "coordinator")
        .expect("claim coordinator");
    plan.start_node(
        producer,
        EntityId::from_bytes([228; 16]),
        EntityId::from_bytes([229; 16]),
        "worker",
        &[],
    )
    .expect("start producer");
    (plan, producer, ready, blocked)
}

/// Runs two assigned siblings concurrently and links profile-tagged results.
#[test]
fn sibling_workers_execute_concurrently_with_distinct_assignments() {
    let session = EntityId::from_bytes([231; 16]);
    let root = EntityId::from_bytes([232; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([233; 16]),
        session,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root },
        "parallel workers",
    )
    .expect("plan");
    for (id, model) in [(234, "wire-a"), (235, "wire-b")] {
        plan.add_node(PlanNode {
            id: EntityId::from_bytes([id; 16]),
            parent: Some(PlanNodeParent::Branch(root)),
            title: model.to_owned(),
            purpose: model.to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: PlanNodeStatus::Planned,
            branch: None,
            result: None,
            superseded_by: None,
            assignment: Some(ModelAssignment {
                provider_id: "provider".to_owned(),
                model_id: model.to_owned(),
                thinking_level: None,
            }),
            claim: None,
            claim_generation: 0,
        })
        .expect("add sibling");
    }
    plan.transition(PlanStatus::Planned).expect("plan plan");
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([236; 16]), "coordinator")
        .expect("claim coordinator");
    let dispatch = plan.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
    let executor = FakeConcurrentWorkers {
        barrier: std::sync::Barrier::new(2),
    };
    let mut next = 240_u8;
    let batch = PlanScheduler::execute_dispatch(
        &mut plan,
        &dispatch,
        &[],
        &executor,
        &worker_models(),
        &scriberig_session::SessionExecutionSelection::default(),
        "fallback-model",
        || {
            next = next.saturating_add(1);
            EntityId::from_bytes([next; 16])
        },
    )
    .expect("execute workers");
    assert_eq!(batch.completed, dispatch);
    assert_eq!(batch.failed, [] as [scriberig_core::EntityId; 0]);
    assert_eq!(batch.results.len(), 2);
    assert_eq!(batch.results[0].result_handles, vec!["provider:provider"]);
    assert_eq!(batch.results[1].result_handles, vec!["provider:provider"]);
    assert!(
        plan.nodes
            .iter()
            .all(|node| node.status == PlanNodeStatus::Completed)
    );
}

/// Persists a candidate merge conflict on the failed worker branch.
#[test]
fn failed_candidate_integration_retains_structured_conflict_code() {
    let (mut plan, _, ready, _) = scheduler_plan();
    let executor = CandidateConflictWorker;
    let mut next = 251_u8;
    let batch = PlanScheduler::execute_dispatch(
        &mut plan,
        &[ready],
        &[],
        &executor,
        &worker_models(),
        &scriberig_session::SessionExecutionSelection::default(),
        "fallback-model",
        || {
            next = next.saturating_add(1);
            EntityId::from_bytes([next; 16])
        },
    )
    .expect("settle failed integration worker");

    assert_eq!(batch.failed, vec![ready]);
    let failure = batch.results.first().expect("durable failure result");
    assert_eq!(failure.status, BranchStatus::Abandoned);
    assert!(
        failure
            .result_handles
            .contains(&"failure-code:candidate_merge_conflict".to_owned())
    );
    assert_eq!(
        failure.summary.as_deref(),
        Some("notes.txt has conflicting updates")
    );
    assert_eq!(
        plan.nodes
            .iter()
            .find(|node| node.id == ready)
            .unwrap()
            .status,
        PlanNodeStatus::Failed
    );
}

/// Synchronizes fake workers so the test proves overlapping execution.
struct FakeConcurrentWorkers {
    /// Barrier reached only after both workers have started.
    barrier: std::sync::Barrier,
}

/// Returns one structured same-target candidate conflict.
struct CandidateConflictWorker;

impl PlanWorkerExecutor for CandidateConflictWorker {
    /// Reports a deterministic conflict without invoking a provider.
    /// # Parameters
    /// - `_node`: Input for this operation.
    /// - `_claim`: Input for this operation.
    /// - `_execution`: Input for this operation.
    /// # Returns
    /// Always a candidate-merge conflict.
    /// # Errors
    /// Returns the structured candidate-merge failure.
    fn execute(
        &self,
        _node: &PlanNode,
        _claim: &WorkerClaim,
        _execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        Err("candidate_merge_conflict: notes.txt has conflicting updates".to_owned())
    }
}

impl PlanWorkerExecutor for FakeConcurrentWorkers {
    /// Returns a compact fake result after both sibling workers start.
    /// # Parameters
    /// - `_node`: Input for this operation.
    /// - `claim`: Input for this operation.
    /// - `execution`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        _node: &PlanNode,
        claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        self.barrier.wait();
        Ok(BranchResult {
            id: EntityId::from_bytes([claim.id.as_bytes()[0].wrapping_add(10); 16]),
            branch: claim.branch,
            status: BranchStatus::Completed,
            summary: Some("fake result".to_owned()),
            result_kind: None,
            candidate_context: None,
            references: Vec::new(),
            result_handles: vec![format!("provider:{}", execution.provider_id)],
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        })
    }
}

/// Dispatches only unclaimed ready nodes within remaining concurrency slots.
#[test]
fn scheduler_respects_capacity_and_dependency_blocking() {
    let (plan, producer, ready, blocked) = scheduler_plan();
    let tick = PlanScheduler::tick(&plan, &[], 2, |_| true);

    assert_eq!(tick.running, vec![producer]);
    assert_eq!(tick.blocked, vec![blocked]);
    assert_eq!(tick.dispatch, vec![ready]);
}

/// Leaves ready work undispatched until trusted assignment resolution succeeds.
#[test]
fn scheduler_skips_unresolved_assignments() {
    let (plan, _, _, blocked) = scheduler_plan();
    let tick = PlanScheduler::tick(&plan, &[], 3, |_| false);

    assert_eq!(tick.blocked, vec![blocked]);
    assert_eq!(tick.dispatch, [] as [scriberig_core::EntityId; 0]);
}

/// Keeps failed dependencies blocked and pauses without claiming new work.
#[test]
fn scheduler_reports_failure_and_honors_pause() {
    let (mut plan, producer, _, blocked) = scheduler_plan();
    let claim = EntityId::from_bytes([229; 16]);
    plan.fail_node(producer, claim).expect("fail producer");
    let failed = PlanScheduler::tick(&plan, &[], 3, |_| true);
    assert_eq!(failed.failed, vec![producer]);
    assert_eq!(failed.blocked, vec![blocked]);

    plan.transition(PlanStatus::Paused).expect("pause plan");
    let paused = PlanScheduler::tick(&plan, &[], 3, |_| true);
    assert!(paused.paused);
    assert_eq!(paused.dispatch, [] as [scriberig_core::EntityId; 0]);
}
