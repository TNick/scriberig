//! Verifies structured plans render in a loaded conversation tree.

use super::tui_pty_support::{TuiPty, test_lock};
use scriberig_session::SessionStore;

/// Verifies a durable plan appears beside its anchored conversation branch.
#[test]
fn tui_pty_shows_a_session_plan_in_the_conversation_tree() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
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
    plan.add_node(scriberig_session::PlanNode {
        id: scriberig_core::EntityId::new(),
        parent: None,
        title: "Stable selected work item".to_owned(),
        purpose: "Remain selected across product view changes".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: scriberig_session::PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
        abandon_reason: None,
        last_failure: None,
    })
    .expect("add durable plan node");
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

    tui.send_slow(format!("/switch {session_id}\r").as_bytes());
    assert!(
        tui.read_until("Historical acceptance plan"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    tui.send_slow(b"plans\r");
    // Only the Plans tree paints this exact header label; the conversation view
    // labels the same durable plan `Plan: Historical acceptance plan`, and its
    // details block already showed the goal before the view switched.
    assert!(
        tui.read_until("Plan · Historical acceptance plan · Planned"),
        "{}",
        tui.diagnostic()
    );
    assert!(
        tui.read_until("Goal: Historical acceptance plan"),
        "{}",
        tui.diagnostic()
    );

    // The conversation view above projects this plan's single root node under its
    // anchored branch, which leaves the shared plan header row childless there. The
    // Plans view reuses that retained tree, so the header must reveal its own node
    // row instead of inheriting the childless shape from the other view.
    let plans_frame = tui.screen();
    let header_line = plans_frame
        .lines()
        .position(|line| line.contains("Plan · Historical acceptance plan · Planned"))
        .unwrap_or_else(|| panic!("the Plans view renders no plan header row:\n{plans_frame}"));
    let node_line = plans_frame
        .lines()
        .position(|line| line.contains("Stable selected work item"))
        .unwrap_or_else(|| {
            panic!(
                "the Plans view renders no node row for the plan's structural root node:\n{plans_frame}"
            )
        });
    assert!(
        node_line > header_line,
        "the node row belongs beneath its plan header row:\n{plans_frame}"
    );
    let node_row = plans_frame.lines().nth(node_line).expect("node row text");
    assert!(
        node_row.contains("root node"),
        "the node row must report its own structural parent category:\n{node_row}"
    );
    assert!(tui.shutdown());
}
