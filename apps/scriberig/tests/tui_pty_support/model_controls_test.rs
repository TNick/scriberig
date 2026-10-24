//! Verifies upstream provider, model, and thinking controls in the live TUI.

use super::{TuiPty, test_lock};
use scriberig_session::SessionStore;

/// Verifies the active terminal path shows upstream selectors for all controls.
#[test]
fn tui_pty_uses_upstream_model_controls() {
    let _lock = test_lock();
    let catalog = r#"{"providers":{"local":{"api":"openai-completions","models":[{"id":"fake-a","name":"Fake A"}]}}}"#;
    let mut tui = TuiPty::start_with_model_catalog(catalog);

    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("F(2) Provider"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1bOQ");
    assert!(tui.read_until("local"), "{}", tui.diagnostic());
    tui.send(b"\x1b[B");
    tui.send(b"\r");
    assert!(tui.read_until("Fake A"), "{}", tui.diagnostic());
    tui.send(b"\x1b");
    assert!(
        tui.read_until_not("Select provider model"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.shutdown());
}

/// Verifies the thinking selector is an upstream list showing the selected model's levels.
///
/// The scenario assigns one upstream model, reads the thinking rows the upstream
/// runtime advertises for it, then reassigns a model whose upstream thinking map
/// removes levels and requires the visible rows to change with it.
#[test]
fn tui_pty_thinking_selector_follows_the_selected_upstream_model() {
    let _lock = test_lock();
    let catalog = r#"{"providers":{"local":{"api":"openai-completions","models":[{"id":"fake-a","name":"Fake A"},{"id":"fake-b","name":"Fake B","thinkingLevelMap":{"low":null,"minimal":null}}]}}}"#;
    let mut tui = TuiPty::start_with_model_catalog(catalog);
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());

    let session = seed_thinking_session(&tui);
    tui.send_slow(format!("/switch {session}\r").as_bytes());
    assert!(
        tui.read_until("Thinking selector conversation"),
        "{}",
        tui.diagnostic()
    );

    tui.send_slow(b"/model\r");
    assert!(tui.read_until("Select model"), "{}", tui.diagnostic());
    tui.send_slow(b"Fake A");
    tui.send(b"\r");
    assert!(tui.read_until("model fake-a"), "{}", tui.diagnostic());

    tui.send_slow(b"/thinking\r");
    assert!(tui.read_until("minimal"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("high"), "{}", tui.diagnostic());
    tui.send(b"\x1b");
    assert!(tui.read_until_not("minimal"), "{}", tui.diagnostic());

    tui.send_slow(b"/model\r");
    assert!(tui.read_until("Select model"), "{}", tui.diagnostic());
    tui.send_slow(b"Fake B");
    tui.send(b"\r");
    assert!(tui.read_until("model fake-b"), "{}", tui.diagnostic());

    tui.send_slow(b"/thinking\r");
    assert!(tui.read_until("high"), "{}", tui.diagnostic());
    assert!(!tui.screen().contains("minimal"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Seeds one durable conversation the running TUI can switch to.
///
/// - `tui`: Running process whose private database receives the fixture.
///
/// The seeded conversation identity.
/// # Returns
/// The new durable session identity.
fn seed_thinking_session(tui: &TuiPty) -> scriberig_core::EntityId {
    let session = scriberig_core::EntityId::new();
    let mut root = scriberig_session::Branch::new(
        scriberig_core::EntityId::new(),
        "Thinking selector conversation",
    )
    .expect("create seeded branch root");
    root.status = scriberig_session::BranchStatus::Completed;
    let store = scriberig_session_sqlite::SqliteSessionStore::open(
        tui.fixture_path().join("scriberig.sqlite"),
    )
    .expect("open PTY session store");
    store
        .save(&scriberig_session::Session {
            id: session,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("seed thinking selector conversation");
    drop(store);
    session
}
