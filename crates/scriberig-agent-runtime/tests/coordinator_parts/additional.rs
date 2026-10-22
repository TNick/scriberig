//! Covers additional focused cases from the parent test module.

use super::*;

/// Runs dependent workers through persisted claims and compact branch results.
#[test]
#[allow(clippy::too_many_lines)] // This durable ordering scenario is intentionally end-to-end.
fn coordinator_persists_claims_results_and_dependency_completion() {
    let service = ApplicationService::new(Backend {
        store: std::sync::Arc::new(MemoryStore::new()),
    })
    .with_session_store(std::sync::Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open_in_memory()
            .expect("open durable session store"),
    ))
    .with_upstream_models(vec![
        rpi_ai::Model::new(
            "wire-a",
            "Model A",
            rpi_ai::Api::AnthropicMessages,
            "provider",
            "https://example.test",
        ),
        rpi_ai::Model::new(
            "wire-b",
            "Model B",
            rpi_ai::Api::AnthropicMessages,
            "provider",
            "https://example.test",
        ),
    ]);
    let session_id = EntityId::from_bytes([1; 16]);
    let root = EntityId::from_bytes([2; 16]);
    let plan_id = EntityId::from_bytes([3; 16]);
    let first = EntityId::from_bytes([4; 16]);
    let parallel = EntityId::from_bytes([5; 16]);
    let second = EntityId::from_bytes([6; 16]);
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
        "complete dependent work",
    )
    .expect("plan");
    for (id, title, assignment) in [
        (
            first,
            "analyze",
            Some(scriberig_session::ModelAssignment {
                provider_id: "provider".to_owned(),
                model_id: "wire-a".to_owned(),
                thinking_level: None,
            }),
        ),
        (
            parallel,
            "document",
            Some(scriberig_session::ModelAssignment {
                provider_id: "provider".to_owned(),
                model_id: "wire-b".to_owned(),
                thinking_level: None,
            }),
        ),
        (second, "integrate", None),
    ] {
        plan.add_node(PlanNode {
            id,
            parent: Some(PlanNodeParent::Branch(root)),
            title: title.to_owned(),
            purpose: title.to_owned(),
            expected_result: Some(scriberig_session::ExpectedResult {
                kind: "worker-summary".to_owned(),
                requires_handle: true,
            }),
            semantic_scope: vec!["entity:known-module".to_owned()],
            status: PlanNodeStatus::Planned,
            branch: None,
            result: None,
            superseded_by: None,
            assignment,
            claim: None,
            claim_generation: 0,
            abandon_reason: None,
            last_failure: None,
        })
        .expect("node");
    }
    plan.add_dependency(first, second).expect("dependency");
    plan.transition(PlanStatus::Planned).expect("planned");
    let start_revision = plan.revision;
    let mut session = Session {
        id: session_id,
        branches: vec![Branch::new(root, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    session.add_plan(plan).expect("attach plan");
    SessionBackend::save_session(&service, &context, &session).expect("persist session");

    let started = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session: session_id,
                plan: plan_id,
                expected_revision: start_revision,
            },
        )
        .expect("start coordinator");
    let coordinator_claim = started
        .coordinator_claim
        .as_ref()
        .expect("durable coordinator claim")
        .id;
    let coordinator_operation = started
        .coordinator_claim
        .as_ref()
        .and_then(|claim| claim.operation.clone())
        .expect("tracked coordinator operation");
    let read_durable = || {
        SessionBackend::read_session(
            &service,
            &context,
            &SessionRequest {
                session: session_id,
            },
        )
        .expect("read session during worker execution")
    };
    let save_durable = |session| {
        SessionBackend::save_session(&service, &context, &session)
            .expect("persist worker private history");
    };
    let mut next = 10_u8;
    let workers = FakeWorkers {
        barrier: std::sync::Barrier::new(2),
        started: std::sync::atomic::AtomicUsize::new(0),
        dependency_context_seen: std::sync::atomic::AtomicBool::new(false),
        read_durable: &read_durable,
        save_durable: &save_durable,
    };
    let results = PlanCoordinator::run_until_idle(
        &service,
        &context,
        session_id,
        plan_id,
        coordinator_claim,
        &workers,
        &runtime_fallback_models(),
        "fallback-model",
        2,
        || {
            next = next.saturating_add(1);
            EntityId::from_bytes([next; 16])
        },
    )
    .expect("advance plan");

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].workers.completed, vec![first, parallel]);
    assert_eq!(results[0].schedule.dispatch, vec![first, parallel]);
    assert_eq!(results[1].workers.completed, vec![second]);
    assert_eq!(results[1].schedule.dispatch, vec![second]);
    assert!(
        workers
            .dependency_context_seen
            .load(std::sync::atomic::Ordering::SeqCst)
    );
    assert_eq!(results[1].plan.status, PlanStatus::Completed);
    assert!(results[1].plan.coordinator_claim.is_none());
    let durable = SessionBackend::read_session(
        &service,
        &context,
        &SessionRequest {
            session: session_id,
        },
    )
    .expect("reload session");
    let analyze_branch = results[0].plan.nodes[0].branch.expect("analyze branch id");
    let document_branch = results[0].plan.nodes[1].branch.expect("document branch id");
    assert_ne!(analyze_branch, document_branch);
    let analyze_result = durable
        .branches
        .iter()
        .find(|branch| branch.id == analyze_branch)
        .and_then(|branch| branch.result.as_ref())
        .expect("analyze result");
    let document_result = durable
        .branches
        .iter()
        .find(|branch| branch.id == document_branch)
        .and_then(|branch| branch.result.as_ref())
        .expect("document result");
    assert!(
        analyze_result
            .result_handles
            .contains(&"model:wire-a".to_owned())
    );
    assert!(
        document_result
            .result_handles
            .contains(&"model:wire-b".to_owned())
    );
    assert_eq!(durable.branches.len(), 4);
    assert_eq!(durable.branches[0].children.len(), 3);
    assert!(
        durable.branches[0]
            .children
            .contains(&results[0].plan.nodes[0].branch.unwrap())
    );
    assert_eq!(durable.plans[0].status, PlanStatus::Completed);
    assert_eq!(
        service
            .operations()
            .get(&coordinator_operation)
            .expect("coordinator operation")
            .status,
        scriberig_app::OperationStatus::Completed
    );
    assert_eq!(
        durable.plans[0].dependencies[0].fulfilled_by,
        durable.plans[0].nodes[0].result
    );
    assert_eq!(
        durable.plans[0]
            .nodes
            .iter()
            .map(|node| node.status)
            .collect::<Vec<_>>(),
        vec![
            PlanNodeStatus::Completed,
            PlanNodeStatus::Completed,
            PlanNodeStatus::Completed
        ]
    );
    assert!(durable.branches[1..].iter().any(|branch| {
        branch.result.as_ref().is_some_and(|result| {
            result
                .result_handles
                .contains(&"entity:api-contract".to_owned())
        })
    }));
}

#[path = "part1.rs"]
mod part1;
