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
        &worker_library(),
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
        profile: "alpha".to_owned(),
        model: "model-a".to_owned(),
        thinking_level: Some("unsupported".to_owned()),
    });
    let executor = CountingWorker(std::sync::atomic::AtomicUsize::new(0));
    let mut next = 240_u8;
    let result = PlanScheduler::execute_dispatch(
        &mut plan,
        &[ready],
        &[],
        &executor,
        &worker_library(),
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
