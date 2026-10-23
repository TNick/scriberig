//! Verifies real-process agent onboarding and durable workspace refresh.

use super::tui_pty_support::{TuiPty, test_lock};
use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_session::{BranchStatus, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use std::time::{Duration, Instant};

/// Verifies failed import after manifest review leaves the project inactive and source unchanged.
#[test]
fn tui_bootstrap_import_failure_does_not_activate_partial_project() {
    let _lock = super::tui_pty_support::test_lock();
    let mut tui = super::tui_pty_support::TuiPty::start_bootstrap_import_failure();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/project\r");
    assert!(
        tui.read_until("No active project is selected."),
        "{}",
        tui.diagnostic()
    );
    tui.send_slow(b"Onboard the current project.\r");
    let session = tui.stored_session_id();
    let checkout = tui.fixture_path().join("scriberig");
    let manifest = std::fs::read(checkout.join("Cargo.toml")).expect("baseline manifest");
    let source = std::fs::read(checkout.join("crates/scriberig-project/src/scan.rs"))
        .expect("baseline scanner");

    wait_for_import_failure(&tui, session);

    tui.send_slow(b"/project\r");
    assert!(
        tui.read_until("No active project is selected."),
        "{}",
        tui.diagnostic()
    );
    assert_eq!(
        std::fs::read(checkout.join("Cargo.toml")).expect("manifest after failure"),
        manifest
    );
    assert_eq!(
        std::fs::read(checkout.join("crates/scriberig-project/src/scan.rs"))
            .expect("scanner after failure"),
        source
    );
    let database = tui.fixture_path().join("scriberig.sqlite");
    let store = SqliteStore::open(&database).expect("product store");
    let projects = store.list_projects().expect("registered scan projects");
    assert_eq!(projects.len(), 1, "scan registration remains inspectable");
    assert!(projects[0].current_snapshot.is_none());
    assert!(
        ProjectService::new(std::sync::Arc::new(
            SqliteStore::open(&database).expect("manifest service store"),
        ))
        .manifest_summary(projects[0].id)
        .expect("persisted reviewed manifest")
        .file_count
            > 0
    );
    assert!(
        ProjectService::new(std::sync::Arc::new(
            SqliteStore::open(&database).expect("project service store"),
        ))
        .workspace(projects[0].id)
        .is_err()
    );

    let saved = tui.load_session(session);
    let branch = saved
        .branches
        .iter()
        .find(|branch| {
            branch
                .result
                .as_ref()
                .is_some_and(|result| result.result_kind.as_deref() == Some("agent-failure"))
        })
        .expect("durable failed onboarding branch");
    assert!(
        branch
            .outcome
            .as_deref()
            .is_some_and(|outcome| { outcome.contains("manifest_changed") }),
        "failure outcome: {:?}",
        branch.outcome
    );
    assert!(tui.shutdown_allow_failure());
}

/// Runs project onboarding through the deterministic provider in a real TUI PTY.
#[test]
fn tui_bootstraps_project_and_reopens_exact_workspace() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_bootstrap();
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/project\r");
    assert!(
        tui.read_until("No active project is selected."),
        "{}",
        tui.diagnostic()
    );
    tui.send_slow(b"Onboard the current project.\r");
    let session = tui.stored_session_id();
    let onboarded = tui.read_until("current project is onboarded");
    let saved = tui.load_session(session);
    assert!(onboarded, "{}; saved session: {saved:?}", tui.diagnostic());
    wait_for_onboarding_completion(&tui, session);

    let fixture = tui.fixture_path().to_path_buf();
    let database = fixture.join("scriberig.sqlite");
    let store = SqliteStore::open(&database).expect("open product database");
    let project = store
        .list_projects()
        .expect("list projects")
        .into_iter()
        .find(|project| project.name == "scriberig")
        .expect("onboarded project");
    let workspace = ProjectService::new(std::sync::Arc::new(
        SqliteStore::open(&database).expect("open project service store"),
    ))
    .workspace(project.id)
    .expect("exact active workspace");
    assert_eq!(project.current_snapshot, Some(workspace.snapshot));
    assert!(
        store
            .project_entities(project.id)
            .expect("project ownership")
            .iter()
            .any(|entity| entity.entity == workspace.root_entity)
    );
    assert_durable_session(&database, session);
    drop(store);
    let fixture = tui.quit_retain_fixture();

    let mut reopened = TuiPty::restart_bootstrap(fixture.clone());
    assert!(
        reopened.read_until("Conversation"),
        "{}",
        reopened.diagnostic()
    );
    assert!(reopened.shutdown());
}

/// Waits until the durable onboarding branch has completed.
///
/// - `tui`: Active native TUI process owning the session database.
/// - `session`: Exact onboarding session identity.
fn wait_for_onboarding_completion(tui: &TuiPty, session: EntityId) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let saved = tui.load_session(session);
        if saved.branches.iter().any(|branch| {
            branch.status == BranchStatus::Completed
                && branch.turns.iter().any(|turn| {
                    turn.agent.as_ref().is_some_and(|message| {
                        message.text.contains("current project is onboarded")
                    })
                })
        }) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "onboarding branch did not complete"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Waits until the invalid reviewed manifest is durably rejected.
///
/// - `tui`: Active PTY owning the database containing the durable session.
/// - `session`: Exact session identity for the bootstrap attempt.
fn wait_for_import_failure(tui: &TuiPty, session: EntityId) {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let saved = tui.load_session(session);
        if saved.branches.iter().any(|branch| {
            branch
                .result
                .as_ref()
                .is_some_and(|result| result.result_kind.as_deref() == Some("agent-failure"))
        }) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "invalid onboarding import did not fail"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Verifies durable onboarding identity, workspace references, and human input.
///
/// - `database`: Temporary SQLite database used by the product process.
/// - `session`: Session identity created by the TUI agent composer.
fn assert_durable_session(database: &std::path::Path, session: EntityId) {
    let session_store = SqliteSessionStore::open(database).expect("open session store");
    let saved_session = session_store
        .load(session)
        .expect("read session")
        .expect("durable onboarding session");
    assert_ne!(saved_session.branches.len(), 0);
    let onboarding = saved_session
        .branches
        .iter()
        .find(|branch| {
            branch
                .turns
                .first()
                .and_then(|turn| turn.human.as_ref())
                .is_some()
        })
        .expect("durable onboarding branch");
    assert_eq!(onboarding.status, BranchStatus::Completed);
    let result_handles = &onboarding
        .result
        .as_ref()
        .expect("durable branch result")
        .result_handles;
    assert!(
        result_handles
            .iter()
            .any(|value| value.starts_with("project:"))
    );
    assert!(
        result_handles
            .iter()
            .any(|value| value.starts_with("snapshot:"))
    );
    assert!(
        result_handles
            .iter()
            .any(|value| value.starts_with("root:"))
    );
    assert!(
        result_handles
            .iter()
            .any(|value| value.starts_with("manifest:"))
    );
    assert_eq!(
        onboarding.turns[0]
            .human
            .as_ref()
            .expect("human instruction")
            .text,
        "Onboard the current project."
    );
    drop(session_store);
}
