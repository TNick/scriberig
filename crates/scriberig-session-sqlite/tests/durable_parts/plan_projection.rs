//! Verifies aggregate plans remain read-only compatibility projections.

use scriberig_core::EntityId;
use scriberig_session::{Branch, ContinuationAnchor, Plan, PlanStatus, Session, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;

/// Ignores stale aggregate plan edits after semantic plan creation.
#[test]
fn stale_session_save_cannot_override_semantic_plan_root() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-plan-projection-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([91; 16]);
    let branch_id = EntityId::from_bytes([92; 16]);
    let plan_id = EntityId::from_bytes([93; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: branch_id },
        "canonical semantic goal",
    )
    .expect("plan");
    let mut stale = Session {
        id: session_id,
        branches: vec![Branch::new(branch_id, "root").expect("branch")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    store.save(&stale).expect("save initial session");

    let mut canonical = store
        .load(session_id)
        .expect("load session")
        .expect("session exists");
    canonical.plans[0]
        .transition(PlanStatus::Planned)
        .expect("advance canonical plan");
    store
        .compare_and_swap_plan(&canonical, plan_id, 0)
        .expect("persist canonical plan revision");

    stale.plans[0].goal = "stale aggregate overwrite".to_owned();
    stale.plans[0].status = PlanStatus::Failed;
    store
        .save(&stale)
        .expect("save unrelated session projection");

    let reloaded = store
        .load(session_id)
        .expect("reload session")
        .expect("session remains present");
    assert_eq!(reloaded.plans[0].goal, "canonical semantic goal");
    assert_eq!(reloaded.plans[0].status, PlanStatus::Planned);
    assert_eq!(reloaded.plans[0].revision, 1);
    drop(store);
    let _ = std::fs::remove_file(path);
}
