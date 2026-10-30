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
    assert_session_payload_omits_plans(&path, session_id);

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
    let reopened = SqliteSessionStore::open(&path).expect("reopen store");
    let reopened_session = reopened
        .load(session_id)
        .expect("load reopened session")
        .expect("session persists");
    assert_eq!(reopened_session.plans[0].goal, "canonical semantic goal");
    assert_eq!(reopened_session.plans[0].status, PlanStatus::Planned);
    assert_session_payload_omits_plans(&path, session_id);
    drop(reopened);
    let _ = std::fs::remove_file(path);
}

/// Persists the public session plan order during a semantic plan mutation.
#[test]
fn semantic_plan_mutation_persists_reordered_plan_projection() {
    let path =
        std::env::temp_dir().join(format!("scriberig-plan-order-{}.sqlite", EntityId::new()));
    let session_id = EntityId::from_bytes([101; 16]);
    let branch_id = EntityId::from_bytes([102; 16]);
    let first_id = EntityId::from_bytes([103; 16]);
    let second_id = EntityId::from_bytes([104; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let first = Plan::new(
        first_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: branch_id },
        "first plan",
    )
    .expect("first plan");
    let second = Plan::new(
        second_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: branch_id },
        "second plan",
    )
    .expect("second plan");
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(branch_id, "root").expect("branch")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![first, second],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    store.save(&session).expect("save session");

    let mut reordered = store
        .load(session_id)
        .expect("load session")
        .expect("session exists");
    reordered.plans.reverse();
    reordered.plans[0]
        .transition(PlanStatus::Planned)
        .expect("advance second plan");
    store
        .compare_and_swap_plan(&reordered, second_id, 0)
        .expect("save changed plan order");

    let loaded = store
        .load(session_id)
        .expect("reload session")
        .expect("session exists");
    assert_eq!(
        loaded.plans.iter().map(|plan| plan.id).collect::<Vec<_>>(),
        vec![second_id, first_id]
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Confirms session payloads contain no embedded plan contents.
///
/// # Parameters
/// - `path`: SQLite database containing the session.
/// - `session`: Stable session identity to inspect.
fn assert_session_payload_omits_plans(path: &std::path::Path, session: EntityId) {
    let connection = rusqlite::Connection::open(path).expect("open payload inspection connection");
    let payload = connection
        .query_row(
            "SELECT payload FROM session_records WHERE session_id=?1",
            rusqlite::params![session.as_bytes().as_slice()],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("session payload");
    let decoded: serde_json::Value = serde_json::from_slice(&payload).expect("payload JSON");
    assert_eq!(decoded["plans"], serde_json::json!([]));
}
