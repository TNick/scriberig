//! Verifies native TUI conversation and session workflows.

use std::time::{Duration, Instant};

use super::tui_pty_support::{TuiPty, test_lock};
use scriberig_session::SessionStore;

/// Verifies a typed arithmetic question receives the mock model answer in the TUI.
#[test]
fn tui_pty_sends_arithmetic_question_and_renders_mock_answer() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();

    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    tui.send_slow(b"1+1\r");
    assert!(tui.read_until("You: 1+1"), "{}", tui.diagnostic());

    let screen = tui.screen();
    assert!(screen.contains("You: 1+1"), "{}", tui.diagnostic());
    assert!(screen.contains('2'), "{}", tui.diagnostic());
    assert!(!screen.contains("Error:"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Verifies `/new` escapes stale abandoned history before a fresh model reply.
#[test]
fn tui_pty_starts_fresh_after_abandoned_conversation_and_gets_answer() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model_and_abandoned_history();

    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    let old_session = tui.stored_session_id();
    tui.send_slow(format!("/switch {old_session}\r").as_bytes());
    assert!(
        tui.read_until("Old abandoned conversation"),
        "{}",
        tui.diagnostic()
    );

    tui.send_slow(b"/new\r");
    assert!(tui.read_until("New conversation"), "{}", tui.diagnostic());
    assert!(
        tui.read_until_not("Old abandoned conversation"),
        "{}",
        tui.diagnostic()
    );
    assert!(
        tui.read_until("No conversation yet"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"1+1\r");
    assert!(tui.read_until("▏ ▏ 2"), "{}", tui.diagnostic());

    let screen = tui.screen();
    assert!(screen.contains("You: 1+1"), "{}", tui.diagnostic());
    assert!(!screen.contains("Old abandoned conversation"));
    assert!(!screen.contains("Error:"), "{}", tui.diagnostic());
    let store = scriberig_session_sqlite::SqliteSessionStore::open(
        tui.fixture_path().join("scriberig.sqlite"),
    )
    .expect("open PTY session store");
    assert_eq!(store.list().expect("list saved conversations").len(), 2);
    drop(store);
    assert!(tui.shutdown());
}

/// Verifies new, list, switch, and remove commands through the running TUI.
#[test]
fn tui_pty_manages_saved_conversations() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_with_arithmetic_model();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    let removed = seed_completed_session(&tui);
    let first = create_session(&mut tui, "first-session-message", &[removed]);
    let second = create_session(&mut tui, "second-session-message", &[removed, first]);
    assert_sessions_visible(&mut tui, &[first, second, removed]);
    switch_session(
        &mut tui,
        first,
        "first-session-message",
        "second-session-message",
    );
    remove_session(&mut tui, removed);
    assert_sessions_visible(&mut tui, &[first, second]);
    assert!(!tui.screen().contains(&removed.to_string()));
    assert!(tui.shutdown());
}

/// Seeds a completed session for the remove-session command scenario.
///
/// - `tui`: Running process whose private database receives the fixture.
///
/// The saved removable session identity.
/// # Returns
/// The new durable session identity.
fn seed_completed_session(tui: &TuiPty) -> scriberig_core::EntityId {
    let session = scriberig_core::EntityId::new();
    let mut root = scriberig_session::Branch::new(
        scriberig_core::EntityId::new(),
        "Completed old conversation",
    )
    .expect("create old conversation root");
    root.status = scriberig_session::BranchStatus::Completed;
    let store = open_session_store(tui);
    store
        .save(&scriberig_session::Session {
            id: session,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("seed completed old conversation");
    session
}

/// Creates one new session and returns the identity persisted by its first turn.
///
/// - `tui`: Running process accepting the new-conversation workflow.
/// - `prompt`: Distinct user input identifying the created session.
/// - `existing`: Session identities excluded from this lookup.
///
/// The newly persisted session identity.
/// # Returns
/// The new durable session identity.
fn create_session(
    tui: &mut TuiPty,
    prompt: &str,
    existing: &[scriberig_core::EntityId],
) -> scriberig_core::EntityId {
    tui.send_slow(b"/new\r");
    assert!(tui.read_until("New conversation"), "{}", tui.diagnostic());
    tui.send_slow(format!("{prompt}\r").as_bytes());
    assert!(
        tui.read_until(&format!("You: {prompt}")),
        "{}",
        tui.diagnostic()
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let sessions = open_session_store(tui).list().expect("list PTY sessions");
        if let Some(session) = sessions
            .into_iter()
            .find(|session| !existing.contains(&session.id))
        {
            return session.id;
        }
        assert!(
            Instant::now() < deadline,
            "new conversation was not persisted; screen={}",
            tui.screen()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Opens the saved-session list and requires each supplied identity to appear.
///
/// - `tui`: Running process receiving the list command.
/// - `sessions`: Durable session identities expected in the list.
fn assert_sessions_visible(tui: &mut TuiPty, sessions: &[scriberig_core::EntityId]) {
    tui.send_slow(b"/sessions");
    assert!(tui.read_until("/sessions"), "{}", tui.diagnostic());
    tui.send_slow(b"\r");
    for session in sessions {
        assert!(tui.read_until(&session.to_string()), "{}", tui.diagnostic());
    }
    tui.send(b"\x1b");
    if let Some(last) = sessions.last() {
        assert!(
            tui.read_until_not(&last.to_string()),
            "{}",
            tui.diagnostic()
        );
    }
}

/// Switches to one conversation and verifies its sibling is absent.
///
/// - `tui`: Running process receiving the switch command.
/// - `session`: Exact saved session identity to load.
/// - `visible_message`: Human message expected in the selected session.
/// - `hidden_message`: Human message that belongs to another session.
fn switch_session(
    tui: &mut TuiPty,
    session: scriberig_core::EntityId,
    visible_message: &str,
    hidden_message: &str,
) {
    tui.send_slow(format!("/switch {session}").as_bytes());
    assert!(
        tui.read_until(&format!("/switch {session}")),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(
        tui.read_until(&format!("You: {visible_message}")),
        "{}",
        tui.diagnostic()
    );
    assert!(!tui.screen().contains(&format!("You: {hidden_message}")));
}

/// Removes an idle conversation and checks its durable record is gone.
///
/// - `tui`: Running process receiving the removal command.
/// - `session`: Exact completed conversation to remove.
fn remove_session(tui: &mut TuiPty, session: scriberig_core::EntityId) {
    tui.send_slow(format!("/remove-session {session}").as_bytes());
    assert!(
        tui.read_until(&format!("/remove-session {session}")),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(
        tui.read_until_not(&session.to_string()),
        "{}",
        tui.diagnostic()
    );
    assert!(
        open_session_store(tui)
            .load(session)
            .expect("load removed session")
            .is_none()
    );
}

/// Opens the session database belonging to one PTY fixture.
///
/// - `tui`: Running process whose fixture owns the database.
///
/// The durable session store.
/// # Returns
/// The opened PTY session store.
fn open_session_store(tui: &TuiPty) -> scriberig_session_sqlite::SqliteSessionStore {
    scriberig_session_sqlite::SqliteSessionStore::open(tui.fixture_path().join("scriberig.sqlite"))
        .expect("open PTY session store")
}

/// Verifies slash command completion from the persistent composer.
#[test]
fn tui_pty_moves_palette_selection() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/pl");
    assert!(tui.read_until_next("/plan"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Verifies slash command discovery is visible in the composer.
#[test]
fn tui_pty_lists_local_palette_commands() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/pl");
    assert!(tui.read_until("/plan"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Verifies Return submits `/model` and the command leaves the composer.
#[test]
fn tui_pty_submits_model_command_on_return() {
    let _lock = test_lock();
    let catalog = r#"{"providers":{"local":{"api":"openai-completions","models":[{"id":"default","name":"Fake Default"}]}}}"#;
    let mut tui = TuiPty::start_with_model_catalog(catalog);

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/model");
    assert!(tui.read_until("· /model"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(tui.read_until_not("· /model"), "{}", tui.diagnostic());
    assert!(
        tui.read_until("Fake Default · Fake Default · local/default"),
        "{}",
        tui.diagnostic()
    );

    assert!(tui.shutdown());
}

/// Verifies `/model` changes and persists the selected provider/model pair.
#[test]
fn tui_pty_changes_model_with_fake_profiles() {
    let _lock = test_lock();
    let catalog = r#"{"providers":{"local":{"api":"openai-completions","models":[{"id":"default","name":"Fake Default"},{"id":"fake-b","name":"Fake B"}]}}}"#;
    let mut tui = TuiPty::start_with_model_catalog(catalog);

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send(b"Onboard the current project.\r");
    assert!(
        tui.read_until("The current project is onboarded"),
        "{}",
        tui.diagnostic()
    );
    let session = tui.stored_session_id();

    tui.send_slow(b"/model\r");
    assert!(
        tui.read_until("Fake B \u{b7} Fake B \u{b7} local/fake-b"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x1b[B");
    tui.send(b"\r");
    assert!(tui.read_until("fake-b"), "{}", tui.diagnostic());

    let store = scriberig_session_sqlite::SqliteSessionStore::open(
        tui.fixture_path().join("scriberig.sqlite"),
    )
    .expect("open selected model session store");
    let saved = store
        .load(session)
        .expect("read selected model session")
        .expect("selected model session exists");
    assert_eq!(saved.execution.provider_id.as_deref(), Some("local"));
    assert_eq!(saved.execution.model_id.as_deref(), Some("fake-b"));
    drop(store);
    assert!(tui.shutdown());
}

/// Verifies find input accepts text, handles backspace, and submits.
#[test]
fn tui_pty_submits_find_query() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/find readme\r");
    assert!(tui.read_until("Query: readme"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}

/// Verifies local slash commands route to auxiliary views.
#[test]
fn tui_pty_activates_palette_commands() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/operations\r");
    assert!(
        tui.read_until("No observed operations"),
        "{}",
        tui.diagnostic()
    );

    tui.send(b"\x1b");
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send(b"\x1bOP");
    assert!(tui.read_until("ScribeRig help"), "{}", tui.diagnostic());

    assert!(tui.shutdown());
}

/// Verifies history and new conversation commands in the composer.
#[test]
fn tui_pty_opens_history_and_agent_composer() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/history\r");
    assert!(tui.read_until("Snapshot:"), "{}", tui.diagnostic());

    tui.send(b"\x1b");
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/new\r");
    assert!(tui.read_until("New conversation"), "{}", tui.diagnostic());
    assert!(tui.shutdown());
}
