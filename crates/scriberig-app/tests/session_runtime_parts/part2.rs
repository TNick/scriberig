//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies application replan preserves history and publishes latest projection.
#[test]
#[allow(clippy::too_many_lines)]
fn application_replan_preserves_history_and_publishes_latest_projection() {
    let service = ApplicationService::new(Fixture).with_upstream_models(session_upstream_models());
    let context = session_context();
    let session = EntityId::from_bytes([111; 16]);
    service
        .start_agent(&context, session, "plan anchor")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("session");
    let initial = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: state.branches[0].id,
                },
                goal: "revise work".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "old".to_owned(),
                    parent_index: None,
                    title: "Old work".to_owned(),
                    purpose: "initial scope".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("plan");
    let started = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: initial.id,
                expected_revision: initial.revision,
            },
        )
        .expect("start");
    let paused = service
        .pause_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: initial.id,
                expected_revision: started.revision,
            },
        )
        .expect("pause");
    let revised = service
        .replan(
            &context,
            &scriberig_app::PlanReplanRequest {
                session,
                plan: initial.id,
                expected_revision: paused.revision,
                additions: vec![PlanNodeDraft {
                    key: "new".to_owned(),
                    parent_index: None,
                    title: "Replacement work".to_owned(),
                    purpose: "updated scope".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
                removed_dependencies: Vec::new(),
                superseded: vec![(initial.nodes[0].id, "new".to_owned())],
                assignment_changes: Vec::new(),
                preserved_terminal_nodes: Vec::new(),
                reason: "updated scope".to_owned(),
            },
        )
        .expect("replan");
    assert_eq!(revised.revision, paused.revision + 1);
    assert_eq!(
        revised.nodes[0].status,
        scriberig_session::PlanNodeStatus::Abandoned
    );
    assert_eq!(revised.nodes[0].superseded_by, Some(revised.nodes[1].id));
    assert_eq!(revised.nodes[1].title, "Replacement work");
    assert!(revised.history.iter().any(|revision| {
        revision
            .nodes
            .iter()
            .any(|node| node.id == initial.nodes[0].id && node.title == "Old work")
    }));
    assert_eq!(
        revised
            .history
            .last()
            .and_then(|revision| revision.reason.as_deref()),
        Some("updated scope")
    );
    let latest = service
        .read_session(&context, &SessionRequest { session })
        .expect("latest session");
    assert_eq!(latest.plans[0], revised);
    assert!(
        service
            .replan(
                &context,
                &scriberig_app::PlanReplanRequest {
                    session,
                    plan: initial.id,
                    expected_revision: paused.revision,
                    additions: vec![PlanNodeDraft {
                        key: "stale".to_owned(),
                        parent_index: None,
                        title: "Stale work".to_owned(),
                        purpose: "must conflict".to_owned(),
                        expected_result: None,
                        semantic_scope: Vec::new(),
                        assignment: None,
                    }],
                    dependencies: Vec::new(),
                    removed_dependencies: Vec::new(),
                    superseded: Vec::new(),
                    assignment_changes: Vec::new(),
                    preserved_terminal_nodes: Vec::new(),
                    reason: "stale proposal".to_owned(),
                },
            )
            .is_err()
    );
}

/// Completes and persists a plan only after its branch result is available.
#[test]
#[allow(clippy::too_many_lines)]
fn application_completion_persists_final_plan_and_releases_operation() {
    let service = ApplicationService::new(Fixture).with_upstream_models(session_upstream_models());
    let context = session_context();
    let session_id = EntityId::from_bytes([118; 16]);
    service
        .start_agent(&context, session_id, "completion anchor")
        .expect("seed session");
    let session = service
        .read_session(
            &context,
            &SessionRequest {
                session: session_id,
            },
        )
        .expect("session");
    let root = session.branches[0].id;
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session: session_id,
                anchor: scriberig_session::ContinuationAnchor::BranchHead { branch: root },
                goal: "complete one node".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "work".to_owned(),
                    parent_index: None,
                    title: "Work".to_owned(),
                    purpose: "finish".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    let running = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session: session_id,
                plan: plan.id,
                expected_revision: plan.revision,
            },
        )
        .expect("start plan");
    let operation = running
        .coordinator_claim
        .as_ref()
        .unwrap()
        .operation
        .clone()
        .unwrap();
    let mut persisted = service
        .read_session(
            &context,
            &SessionRequest {
                session: session_id,
            },
        )
        .expect("running session");
    let worker_branch = EntityId::from_bytes([119; 16]);
    let worker_claim = EntityId::from_bytes([120; 16]);
    let result = scriberig_session::BranchResult {
        id: EntityId::from_bytes([121; 16]),
        branch: worker_branch,
        status: scriberig_session::BranchStatus::Completed,
        summary: Some("done".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec!["result:work".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    let durable_branch =
        scriberig_session::Branch::new(worker_branch, "worker").expect("worker branch");
    let mut durable_branch = durable_branch;
    durable_branch.status = scriberig_session::BranchStatus::Completed;
    durable_branch.result = Some(result.clone());
    persisted.branches.push(durable_branch);
    let active_plan = persisted
        .plans
        .iter_mut()
        .find(|entry| entry.id == plan.id)
        .unwrap();
    active_plan
        .start_node(plan.nodes[0].id, worker_branch, worker_claim, "worker", &[])
        .expect("claim worker");
    active_plan
        .complete_node(plan.nodes[0].id, worker_claim, &result)
        .expect("link result");
    let observed_revision = active_plan.revision;
    service
        .save_session(&context, &persisted)
        .expect("persist worker completion");
    let completed = service
        .complete_plan(
            &context,
            &PlanLifecycleRequest {
                session: session_id,
                plan: plan.id,
                expected_revision: observed_revision,
            },
        )
        .expect("complete plan");
    assert_eq!(completed.status, scriberig_session::PlanStatus::Completed);
    assert!(completed.coordinator_claim.is_none());
    assert_eq!(
        service.operations().get(&operation).unwrap().status,
        scriberig_app::OperationStatus::Completed
    );
}

/// Resolves sibling plan assignments independently through the upstream catalog.
#[test]
fn plan_assignments_resolve_distinct_models_and_reject_unsupported_thinking() {
    let session_default = scriberig_session::SessionExecutionSelection::default();
    let node =
        |provider_id: &str, model_id: &str, thinking: Option<&str>| scriberig_session::PlanNode {
            id: EntityId::from_bytes([111; 16]),
            parent: None,
            title: model_id.to_owned(),
            purpose: "assigned work".to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: scriberig_session::PlanNodeStatus::Ready,
            branch: None,
            result: None,
            superseded_by: None,
            abandon_reason: None,
            last_failure: None,
            assignment: Some(scriberig_session::ModelAssignment {
                provider_id: provider_id.to_owned(),
                model_id: model_id.to_owned(),
                thinking_level: thinking.map(str::to_owned),
            }),
            claim: None,
            claim_generation: 0,
        };
    let first = scriberig_app::resolve_plan_node_execution(
        &node("local", "native-a", Some("high")),
        &session_default,
        &session_upstream_models(),
        "fallback",
    )
    .expect("resolve first worker");
    let second = scriberig_app::resolve_plan_node_execution(
        &node("local", "native-b", Some("low")),
        &session_default,
        &session_upstream_models(),
        "fallback",
    )
    .expect("resolve second worker");
    assert_eq!(first.model_id, "native-a");
    assert_eq!(second.model_id, "native-b");
    assert_eq!(first.provider_id, "local");
    assert!(
        scriberig_app::resolve_plan_node_execution(
            &node("local", "native-a", Some("not-advertised")),
            &session_default,
            &session_upstream_models(),
            "fallback",
        )
        .is_err()
    );
    assert!(
        scriberig_app::resolve_plan_node_execution(
            &node("not-advertised", "native-a", Some("high")),
            &session_default,
            &session_upstream_models(),
            "fallback",
        )
        .is_err()
    );
    assert!(
        scriberig_app::resolve_plan_node_execution(
            &node("local", "not-advertised", Some("high")),
            &session_default,
            &session_upstream_models(),
            "fallback",
        )
        .is_err()
    );
}

/// Verifies an agent command cannot bypass the application capability boundary.
#[test]
fn agent_submission_without_capability_is_denied() {
    let service = ApplicationService::new(Fixture);
    let error = service
        .start_agent(
            &ApplicationContext::empty(),
            EntityId::from_bytes([4; 16]),
            "denied",
        )
        .expect_err("agent capability must be required");

    assert!(matches!(
        error,
        ApplicationError::PermissionDenied { action, .. } if action == "agent_start"
    ));
}

#[path = "part2/failures.rs"]
mod failures;
#[path = "part2/plan_attempt_history.rs"]
mod plan_attempt_history;
#[path = "part2/plan_decompose.rs"]
mod plan_decompose;
#[path = "part2/plan_next_step.rs"]
mod plan_next_step;
#[path = "part2/plan_node_details.rs"]
mod plan_node_details;
#[path = "part2/plan_retry.rs"]
mod plan_retry;
#[path = "part2/plan_rollups.rs"]
mod plan_rollups;
#[path = "part2/plan_semantic_mutations.rs"]
mod plan_semantic_mutations;
