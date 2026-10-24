//! Verifies provider-bound scheduler claim and assignment behavior.

use super::*;
/// Persists worker ownership before crossing the provider boundary.
#[test]
fn scheduler_persists_claims_before_worker_execution() {
    let (mut plan, _, ready, _) = scheduler_plan();
    let executor = CountingWorker(std::sync::atomic::AtomicUsize::new(0));
    let mut next = 250_u8;
    let persisted = std::sync::atomic::AtomicBool::new(false);
    let _ = PlanScheduler::execute_dispatch_persisted(
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
        |claimed| {
            let node = claimed
                .nodes
                .iter()
                .find(|node| node.id == ready)
                .expect("claimed node");
            assert_eq!(node.status, PlanNodeStatus::Running);
            assert!(node.claim.is_some());
            assert_eq!(
                executor.0.load(std::sync::atomic::Ordering::SeqCst),
                0,
                "worker cannot start before claim persistence"
            );
            persisted.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
    );

    assert!(persisted.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(executor.0.load(std::sync::atomic::Ordering::SeqCst), 1);
}

/// Rejects unsupported thinking levels before entering the provider boundary.
#[test]
fn unsupported_thinking_fails_before_worker_provider_invocation() {
    let (mut plan, _, ready, _) = scheduler_plan();
    plan.nodes
        .iter_mut()
        .find(|node| node.id == ready)
        .expect("ready node")
        .assignment = Some(ModelAssignment {
        provider_id: "provider".to_owned(),
        model_id: "wire-a".to_owned(),
        thinking_level: Some("unsupported".to_owned()),
    });
    let executor = CountingWorker(std::sync::atomic::AtomicUsize::new(0));
    let mut next = 240_u8;
    let result = PlanScheduler::execute_dispatch(
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
    );

    assert!(result.is_err());
    assert_eq!(executor.0.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        plan.nodes
            .iter()
            .find(|node| node.id == ready)
            .unwrap()
            .status,
        PlanNodeStatus::Ready
    );
}

/// Counts calls crossing the worker and provider boundary.
struct CountingWorker(std::sync::atomic::AtomicUsize);

impl PlanWorkerExecutor for CountingWorker {
    /// Records a provider-bound invocation if dispatch reaches this executor.
    /// # Parameters
    /// - `_node`: Input for this operation.
    /// - `_claim`: Input for this operation.
    /// - `_execution`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        _node: &PlanNode,
        _claim: &WorkerClaim,
        _execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err("unused test worker".to_owned())
    }
}

/// Records each resolved model assignment that reaches a worker.
///
/// # Public members
/// - `executed`: Number of provider-bound invocations that started.
/// - `models`: Provider-native model ID resolved for each started worker.
///
/// # Private members
/// None.
struct RecordingWorker {
    /// Provider-bound invocations that reached this executor.
    executed: std::sync::atomic::AtomicUsize,
    /// Model ID resolved for each invocation that reached this executor.
    models: std::sync::Mutex<Vec<String>>,
}

impl RecordingWorker {
    /// Creates an executor that has not run a worker yet.
    ///
    /// The recording worker for one fixture run.
    /// # Returns
    /// A fresh recording executor.
    fn new() -> Self {
        Self {
            executed: std::sync::atomic::AtomicUsize::new(0),
            models: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Reports the model IDs resolved for every invocation that started.
    ///
    /// The recorded provider-native model identifiers.
    /// # Returns
    /// The recorded model IDs.
    /// # Panics
    /// Panics when the recording lock is poisoned.
    fn resolved_models(&self) -> Vec<String> {
        self.models.lock().expect("worker resolution lock").clone()
    }
}

impl PlanWorkerExecutor for RecordingWorker {
    /// Records the exact resolved assignment and fails the worker safely.
    ///
    /// The worker never produces a durable result here; the recorded assignment
    /// is the behavior under test.
    /// # Parameters
    /// - `_node`: Input for this operation.
    /// - `_claim`: Input for this operation.
    /// - `execution`: Resolved upstream assignment for this worker.
    /// # Returns
    /// A safe worker failure after recording the resolved model.
    /// # Errors
    /// Always returns a safe worker failure.
    fn execute(
        &self,
        _node: &PlanNode,
        _claim: &WorkerClaim,
        execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        self.executed
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.models
            .lock()
            .expect("worker resolution lock")
            .push(execution.model_id.clone());
        Err("recording worker performs no provider work".to_owned())
    }
}

/// Returns the upstream rows published by a reload that removed `wire-a`.
///
/// The post-reload catalog rows used by the stale-assignment fixtures.
/// # Returns
/// Every fixture model except the removed `wire-a` row.
fn reloaded_models_without_wire_a() -> Vec<Model> {
    worker_models()
        .into_iter()
        .filter(|model| model.id != "wire-a")
        .collect()
}

/// Stores one durable assignment on a fixture plan node.
///
/// - `plan`: Plan whose node receives the assignment.
/// - `node_id`: Node to assign.
/// - `assignment`: Safe upstream provider/model/thinking assignment.
fn assign_node(plan: &mut Plan, node_id: EntityId, assignment: ModelAssignment) {
    let node = plan
        .nodes
        .iter_mut()
        .find(|node| node.id == node_id)
        .expect("fixture node");
    node.assignment = Some(assignment);
}

/// Fails a whole dispatch closed when a reload removed an assigned pair.
///
/// The durable assignment must stay untouched and no worker claim or provider
/// invocation may start, so an operator can re-assign the node safely.
#[test]
fn reload_removed_assignment_fails_before_any_claim_or_provider_call() {
    let (mut plan, _, ready, _) = scheduler_plan();
    assign_node(
        &mut plan,
        ready,
        ModelAssignment {
            provider_id: "provider".to_owned(),
            model_id: "wire-a".to_owned(),
            thinking_level: None,
        },
    );
    let before = plan.clone();
    let executor = CountingWorker(std::sync::atomic::AtomicUsize::new(0));
    let persisted = std::sync::atomic::AtomicBool::new(false);
    let mut next = 190_u8;

    let result = PlanScheduler::execute_dispatch_persisted(
        &mut plan,
        &[ready],
        &[],
        &executor,
        &reloaded_models_without_wire_a(),
        &scriberig_session::SessionExecutionSelection::default(),
        "fallback-model",
        || {
            next = next.saturating_add(1);
            EntityId::from_bytes([next; 16])
        },
        |_| {
            persisted.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
    );

    assert!(
        result.is_err(),
        "an assignment naming a removed model must not resolve: {result:?}"
    );
    assert!(
        !persisted.load(std::sync::atomic::Ordering::SeqCst),
        "no worker claim may be persisted for an unresolvable assignment"
    );
    assert_eq!(executor.0.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        plan, before,
        "a rejected dispatch must leave the durable plan and its assignment untouched"
    );
}

/// Re-resolves the same node after reassignment to the reloaded generation.
#[test]
fn reassigned_node_resolves_against_the_reloaded_generation() {
    let (mut plan, _, ready, _) = scheduler_plan();
    assign_node(
        &mut plan,
        ready,
        ModelAssignment {
            provider_id: "provider".to_owned(),
            model_id: "wire-a".to_owned(),
            thinking_level: None,
        },
    );
    let reloaded = reloaded_models_without_wire_a();
    let executor = RecordingWorker::new();
    let mut next = 170_u8;
    let rejected = PlanScheduler::execute_dispatch(
        &mut plan,
        &[ready],
        &[],
        &executor,
        &reloaded,
        &scriberig_session::SessionExecutionSelection::default(),
        "fallback-model",
        || {
            next = next.saturating_add(1);
            EntityId::from_bytes([next; 16])
        },
    );
    assert!(rejected.is_err());

    assign_node(
        &mut plan,
        ready,
        ModelAssignment {
            provider_id: "provider".to_owned(),
            model_id: "wire-b".to_owned(),
            thinking_level: None,
        },
    );
    let accepted = PlanScheduler::execute_dispatch(
        &mut plan,
        &[ready],
        &[],
        &executor,
        &reloaded,
        &scriberig_session::SessionExecutionSelection::default(),
        "fallback-model",
        || {
            next = next.saturating_add(1);
            EntityId::from_bytes([next; 16])
        },
    )
    .expect("the reassigned node resolves against the new generation");

    assert_eq!(
        executor.resolved_models(),
        vec!["wire-b".to_owned()],
        "the worker must run on the reassigned upstream model"
    );
    assert_eq!(accepted.failed, vec![ready]);
}

/// Keeps a reload-stale session default from silently selecting the fallback.
#[test]
fn stale_session_default_does_not_fall_back_to_the_runtime_model() {
    let (mut plan, _, ready, _) = scheduler_plan();
    let stale_session_default = scriberig_session::SessionExecutionSelection {
        provider_id: Some("provider".to_owned()),
        model_id: Some("wire-a".to_owned()),
        ..Default::default()
    };
    let executor = CountingWorker(std::sync::atomic::AtomicUsize::new(0));
    let mut next = 150_u8;

    let result = PlanScheduler::execute_dispatch(
        &mut plan,
        &[ready],
        &[],
        &executor,
        &reloaded_models_without_wire_a(),
        &stale_session_default,
        "fallback-model",
        || {
            next = next.saturating_add(1);
            EntityId::from_bytes([next; 16])
        },
    );

    assert!(
        result.is_err(),
        "a stale session default must fail instead of substituting the runtime model"
    );
    assert_eq!(executor.0.load(std::sync::atomic::Ordering::SeqCst), 0);
}
