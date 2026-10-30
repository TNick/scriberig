//! Verifies stale materialized plan state cannot dispatch gated work.

use super::*;
use rusqlite::params;
use scriberig_agent_runtime::scheduler::PlanWorkerExecutor;
use scriberig_session::{PlanGate, PlanGatePhase, PlanGateRequirement, PlanNodeParent};
use scriberig_session_sqlite::SqliteSessionStore;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Proves a stale cache reporting Ready does not bypass a canonical entry gate.
#[test]
#[allow(clippy::too_many_lines)] // It exercises cache corruption through real dispatch.
fn stale_rollup_status_cannot_dispatch_pending_gate_work() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-stale-rollup-dispatch-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([241; 16]);
    let root_id = EntityId::from_bytes([242; 16]);
    let plan_id = EntityId::from_bytes([243; 16]);
    let node_id = EntityId::from_bytes([244; 16]);
    let coordinator_id = EntityId::from_bytes([245; 16]);
    let store = std::sync::Arc::new(SqliteSessionStore::open(&path).expect("open SQLite store"));
    let service = ApplicationService::new(Backend {
        store: std::sync::Arc::new(MemoryStore::new()),
    })
    .with_session_store(store);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("read capability"),
            CapabilityId::new("agent_plan").expect("plan capability"),
        ]),
        ..ApplicationContext::empty()
    };
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: root_id },
        "do not dispatch through stale cache",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_id)),
        title: "gated work".to_owned(),
        purpose: "wait for exact validation evidence".to_owned(),
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
    .expect("node");
    plan.add_gate(PlanGate {
        id: EntityId::from_bytes([246; 16]),
        owner: node_id,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::ValidationPassed {
            profile: Some("fresh-validation".to_owned()),
        },
        waiver: None,
    })
    .expect("entry gate");
    plan.transition(PlanStatus::Planned).expect("planned");
    plan.start_with_coordinator_claim(coordinator_id, "coordinator", None)
        .expect("running plan");
    SessionBackend::save_session(
        &service,
        &context,
        &Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        },
    )
    .expect("persist gated plan");
    let stored = SessionBackend::read_session(
        &service,
        &context,
        &SessionRequest {
            session: session_id,
        },
    )
    .expect("read canonical plan");
    let snapshot = stored.plans[0].source_snapshot.expect("plan snapshot");
    let connection = rusqlite::Connection::open(&path).expect("open cache connection");
    connection
        .execute(
            "UPDATE plan_node_index SET direct_status='ready'
             WHERE session_id=?1 AND plan_id=?2 AND node_id=?3",
            params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice(),
                node_id.as_bytes().as_slice()
            ],
        )
        .expect("corrupt direct status cache");
    connection
        .execute(
            "UPDATE plan_node_rollup_index SET effective_status='ready'
             WHERE session_id=?1 AND plan_id=?2 AND node_id=?3",
            params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice(),
                node_id.as_bytes().as_slice()
            ],
        )
        .expect("corrupt effective status cache");
    let cached = service
        .read_plan_rollups(&context, session_id, plan_id, snapshot)
        .expect("read deliberately stale status cache");
    assert_eq!(cached[&node_id].direct_status, PlanNodeStatus::Ready);
    assert_eq!(cached[&node_id].effective_status, PlanNodeStatus::Ready);

    let executor = NeverRunExecutor {
        calls: AtomicUsize::new(0),
    };
    let mut next_id = || EntityId::new();
    let advanced = PlanCoordinator::advance_once(
        &service,
        &context,
        session_id,
        plan_id,
        coordinator_id,
        &executor,
        &[],
        "unused-model",
        1,
        &mut next_id,
    )
    .expect("coordinator checks canonical gate");

    assert_eq!(advanced.schedule.dispatch, [] as [EntityId; 0]);
    assert!(advanced.schedule.blocked.contains(&node_id));
    assert_eq!(executor.calls.load(Ordering::SeqCst), 0);
    drop(connection);
    drop(service);
    let _ = std::fs::remove_file(path);
}

/// Records and rejects any attempt to execute gated work.
struct NeverRunExecutor {
    /// Number of worker invocations attempted by the coordinator.
    calls: AtomicUsize,
}

impl PlanWorkerExecutor for NeverRunExecutor {
    /// Fails if stale derived state causes a worker dispatch.
    ///
    /// # Parameters
    /// - `_node`: Canonical plan node selected for execution.
    /// - `_claim`: Worker claim allocated for execution.
    /// - `_execution`: Resolved worker model assignment.
    ///
    /// # Returns
    /// No result; this executor exists only to detect an unsafe invocation.
    ///
    /// # Errors
    /// Always returns an error after recording the unexpected invocation.
    fn execute(
        &self,
        _node: &PlanNode,
        _claim: &WorkerClaim,
        _execution: &scriberig_app::ResolvedPlanExecution,
    ) -> Result<BranchResult, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err("pending gate work must not execute".to_owned())
    }
}
