//! Verifies historical replies create durable recursive branches in the TUI.

use std::time::{Duration, Instant};

use super::tui_pty_support::{TuiPty, test_lock};
use scriberig_session::SessionStore;

/// Verifies a reply to an earlier assistant turn creates an anchored branch.
///
/// The PTY scenario spans startup, durable seeding, interaction, persistence,
/// and recursive rendering, so its end-to-end ordering stays visible together.
#[allow(clippy::too_many_lines)]
#[test]
fn tui_pty_continues_from_a_historical_reply_anchor() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();
    let session_id = scriberig_core::EntityId::new();
    let root_id = scriberig_core::EntityId::new();
    let first_turn = scriberig_core::EntityId::new();
    let later_turn = scriberig_core::EntityId::new();
    let mut root = scriberig_session::Branch::new(root_id, "Prior conversation")
        .expect("create seeded conversation");
    root.push_turn(scriberig_session::Turn {
        id: first_turn,
        ordinal: 0,
        human: Some(scriberig_session::HumanInput {
            text: "first historical question".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "first historical answer".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    root.push_turn(scriberig_session::Turn {
        id: later_turn,
        ordinal: 1,
        human: Some(scriberig_session::HumanInput {
            text: "later question".to_owned(),
        }),
        agent: Some(scriberig_session::AgentOutput {
            text: "later answer".to_owned(),
        }),
        tool_invocation: None,
        tool_result: None,
    });
    root.status = scriberig_session::BranchStatus::Completed;
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
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("seed historical conversation");
    drop(store);

    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    tui.send_slow(format!("/switch {session_id}").as_bytes());
    tui.send(b"\r");
    assert!(
        tui.read_until("You: first historical question"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b");
    assert!(
        tui.read_until_next("Enter pin reply target"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[B\x1b[B");
    tui.send(b"\r");
    tui.send(b"1+1\r");
    assert!(tui.read_until("▏ ▏ 2"), "{}", tui.diagnostic());

    let store = scriberig_session_sqlite::SqliteSessionStore::open(
        tui.fixture_path().join("scriberig.sqlite"),
    )
    .expect("open durable PTY session store");
    let deadline = Instant::now() + Duration::from_secs(30);
    let child = loop {
        let session = store
            .load(session_id)
            .expect("load durable session")
            .expect("durable session exists");
        if let Some(child) = session.branches.iter().find(|branch| {
            branch.status == scriberig_session::BranchStatus::Completed
                && branch.continuation.is_some_and(|origin| {
                    origin.anchor
                        == scriberig_session::ContinuationAnchor::AssistantOutput {
                            branch: root_id,
                            turn: first_turn,
                        }
                })
        }) {
            assert_eq!(child.parent, Some(root_id));
            assert_eq!(child.children, Vec::<scriberig_core::EntityId>::new());
            break child.clone();
        }
        assert!(
            Instant::now() < deadline,
            "historical continuation not persisted"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(
        child
            .turns
            .iter()
            .any(|turn| { turn.human.as_ref().is_some_and(|human| human.text == "1+1") })
    );
    drop(store);

    tui.send(b"\x1b");
    assert!(
        tui.read_until_next("Enter pin reply target"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[B\x1b[B");
    tui.send(b"\x1b[C");
    assert!(
        tui.read_until("Live ModelInvocation"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[D");
    assert!(
        tui.read_until_next("Live ModelInvocation"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}
