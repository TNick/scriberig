//! Verifies upstream provider, model, and thinking controls in the live TUI.

use std::time::{Duration, Instant};

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

/// Verifies the thinking selector is an upstream list and a model switch clamps its level.
///
/// The scenario assigns one upstream model, commits the `low` row that the upstream
/// runtime advertises for it, then reassigns a model whose upstream `thinkingLevelMap`
/// nulls `minimal` and `low`. The durable session selection and the upstream footer
/// must then carry the clamped level, and the reopened list must offer only levels the
/// reassigned model advertises.
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
    assert!(tui.screen().contains("low"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("high"), "{}", tui.diagnostic());
    // The upstream `SelectList` cursor starts on the first advertised level and paints
    // it with the `\u{2192}` marker, so two Down keys move it onto the `low` row that the
    // reassigned model cannot accept.
    tui.send(b"\x1b[B");
    tui.send(b"\x1b[B");
    assert!(tui.read_until("\u{2192} low"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(tui.read_until("thinking low"), "{}", tui.diagnostic());

    let selected = wait_for_selection(&tui, session, Some("fake-a"), Some("low"));
    assert_eq!(selected.execution.provider_id.as_deref(), Some("local"));

    tui.send_slow(b"/model\r");
    assert!(tui.read_until("Select model"), "{}", tui.diagnostic());
    tui.send_slow(b"Fake B");
    tui.send(b"\r");
    assert!(tui.read_until("model fake-b"), "{}", tui.diagnostic());

    // Upstream `Model::clamp_thinking_level` retains a level only when the newly
    // selected model supports it and otherwise falls to the nearest lower supported
    // level. `fake-b` nulls `minimal` and `low`, so the durable selection must move to
    // `off`, the level `fake-b` still advertises at or below `low`.
    let clamped = wait_for_selection(&tui, session, Some("fake-b"), Some("off"));
    assert_eq!(clamped.execution.provider_id.as_deref(), Some("local"));

    assert!(tui.read_until("thinking off"), "{}", tui.diagnostic());
    tui.send_slow(b"/thinking\r");
    assert!(tui.read_until("\u{2192} off"), "{}", tui.diagnostic());
    assert!(tui.screen().contains("high"), "{}", tui.diagnostic());
    assert!(!tui.screen().contains("minimal"), "{}", tui.diagnostic());
    assert!(!tui.screen().contains("low"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Reads the durable session selection written by the running TUI.
///
/// - `tui`: Active PTY whose private database the TUI mutates.
/// - `session`: Seeded conversation the TUI switched to.
/// - `model`: Expected upstream model identity, or `None` when unset.
/// - `thinking_level`: Expected selected thinking level, or `None` when unset.
///
/// # Panics
/// Panics when the database cannot be read or the expected selection is not
/// persisted within 20 seconds.
/// # Returns
/// The persisted durable session.
#[must_use]
fn wait_for_selection(
    tui: &TuiPty,
    session: scriberig_core::EntityId,
    model: Option<&str>,
    thinking_level: Option<&str>,
) -> scriberig_session::Session {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let saved = tui.load_session(session);
        if saved.execution.model_id.as_deref() == model
            && saved.execution.thinking_level.as_deref() == thinking_level
        {
            return saved;
        }
        assert!(
            Instant::now() < deadline,
            "selection was not persisted: expected model {model:?} and thinking level \
             {thinking_level:?}, got {:?}",
            saved.execution
        );
        std::thread::sleep(Duration::from_millis(50));
    }
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
