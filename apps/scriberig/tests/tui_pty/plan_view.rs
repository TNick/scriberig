//! Verifies structured plans render in a loaded conversation tree.

use super::tui_pty_support::{TuiPty, test_lock};
use scriberig_session::SessionStore;

/// Verifies a durable plan appears beside its anchored conversation branch.
#[test]
fn tui_pty_shows_a_session_plan_in_the_conversation_tree() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();
    let session_id = scriberig_core::EntityId::new();
    let root_id = scriberig_core::EntityId::new();
    let mut root = scriberig_session::Branch::new(root_id, "Planned conversation")
        .expect("create seeded conversation");
    root.status = scriberig_session::BranchStatus::Completed;
    let mut plan = scriberig_session::Plan::new(
        scriberig_core::EntityId::new(),
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "Historical acceptance plan",
    )
    .expect("create structured plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("mark plan planned");

    let store = scriberig_session_sqlite::SqliteSessionStore::open(
        tui.fixture_path().join("scriberig.sqlite"),
    )
    .expect("open PTY session store");
    store
        .save(&scriberig_session::Session {
            id: session_id,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("seed planned conversation");
    drop(store);

    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    tui.send_slow(format!("/switch {session_id}").as_bytes());
    tui.send(b"\r");
    assert!(
        tui.read_until("Plan: Historical acceptance plan · Planned"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}
