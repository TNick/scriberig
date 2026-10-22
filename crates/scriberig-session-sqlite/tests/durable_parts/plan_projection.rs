//! Verifies aggregate session saves cannot authoritatively replace plans.

use super::*;

/// Keeps semantic-root plan state authoritative over stale session projections.
#[test]
fn whole_session_save_cannot_overwrite_semantic_plan_revision() {
    let session_id = EntityId::from_bytes([61; 16]);
    let root_branch = EntityId::from_bytes([62; 16]);
    let plan_id = EntityId::from_bytes([63; 16]);
    let store = SqliteSessionStore::open_in_memory().expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "canonical plan goal",
    )
    .expect("create plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan plan");
    plan.transition(scriberig_session::PlanStatus::Running)
        .expect("start plan");
    let initial_revision = plan.revision;
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_branch, "root").expect("branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial session projection");

    let mut stale = store
        .load(session_id)
        .expect("load stale projection")
        .expect("stored session");
    let mut accepted = stale.clone();
    accepted.plans[0]
        .transition(scriberig_session::PlanStatus::Paused)
        .expect("pause canonical plan");
    store
        .compare_and_swap_plan(&accepted, plan_id, initial_revision)
        .expect("promote canonical plan mutation");
    stale.plans[0].goal = "unauthorized aggregate overwrite".to_owned();
    store
        .save(&stale)
        .expect("save unrelated session projection");

    let reloaded = store
        .load(session_id)
        .expect("reload semantic session")
        .expect("stored session");
    assert_eq!(reloaded.plans[0].goal, "canonical plan goal");
    assert_eq!(
        reloaded.plans[0].status,
        scriberig_session::PlanStatus::Paused
    );
    assert_eq!(reloaded.plans[0].revision, initial_revision + 1);
}

/// Resolves an earlier plan revision by its exact semantic snapshot identity.
#[test]
fn historical_plan_snapshot_loads_exact_revision() {
    let session_id = EntityId::from_bytes([64; 16]);
    let root_branch = EntityId::from_bytes([65; 16]);
    let plan_id = EntityId::from_bytes([66; 16]);
    let store = SqliteSessionStore::open_in_memory().expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "historical goal",
    )
    .expect("create plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan plan");
    plan.transition(scriberig_session::PlanStatus::Running)
        .expect("start plan");
    let mut root = Branch::new(root_branch, "root").expect("branch");
    root.result = Some(scriberig_session::BranchResult {
        id: EntityId::from_bytes([67; 16]),
        branch: root_branch,
        status: scriberig_session::BranchStatus::Completed,
        summary: Some("accepted result".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    store
        .save(&Session {
            id: session_id,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial plan");

    let mut initial_session = store
        .load(session_id)
        .expect("load initial session")
        .expect("stored session");
    let initial = initial_session.plans.remove(0);
    let snapshot = initial.source_snapshot.expect("initial root snapshot");
    let mut next = store
        .load(session_id)
        .expect("load current session")
        .expect("stored session");
    next.plans[0]
        .transition(scriberig_session::PlanStatus::Paused)
        .expect("pause current plan");
    store
        .compare_and_swap_plan(&next, plan_id, initial.revision)
        .expect("promote next plan revision");

    let historical = store
        .load_plan_at_snapshot(session_id, plan_id, snapshot)
        .expect("resolve historical plan")
        .expect("snapshot belongs to plan history");
    let mut current_session = store
        .load(session_id)
        .expect("load current session")
        .expect("stored session");
    let current = current_session.plans.remove(0);
    assert_eq!(historical.status, scriberig_session::PlanStatus::Running);
    assert_eq!(historical.revision, initial.revision);
    assert_eq!(historical.source_snapshot, Some(snapshot));
    assert_eq!(current.status, scriberig_session::PlanStatus::Paused);
    let compact = scriberig_session::repository::SessionStore::load_plan_projection(
        &store,
        session_id,
        plan_id,
        Some(snapshot),
    )
    .expect("load compact historical projection")
    .expect("compact snapshot belongs to plan history");
    assert_eq!(compact.status, scriberig_session::PlanStatus::Running);
    assert_eq!(compact.source_snapshot, Some(snapshot));
    assert!(compact.history.is_empty());
    let branch_results =
        scriberig_session::repository::SessionStore::load_plan_branch_results(&store, session_id)
            .expect("load plan branch results")
            .expect("session contains branch results");
    assert_eq!(branch_results.len(), 1);
    assert_eq!(branch_results[0].id, EntityId::from_bytes([67; 16]));
}
