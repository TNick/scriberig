//! Verifies real-process agent onboarding and durable workspace refresh.

use super::tui_pty_support::{TuiPty, test_lock};
use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_session::{BranchStatus, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;

/// Runs project onboarding through the deterministic provider in a real TUI PTY.
#[test]
fn tui_bootstraps_project_and_reopens_exact_workspace() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_bootstrap();
    assert!(
        tui.read_until("project: unselected"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"start-agent\r");
    assert!(tui.read_until("Session:"), "{}", tui.diagnostic());
    let session = tui
        .screen()
        .split("Session:")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .expect("session visible in composer")
        .parse::<EntityId>()
        .expect("session identity");
    tui.send(b"Onboard the current project.\r");
    assert!(
        tui.read_until("The current project is onboarded"),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.read_until("project: scriberig"), "{}", tui.diagnostic());

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
        reopened.read_until("scriberig @ snp_"),
        "{}",
        reopened.diagnostic()
    );
    assert!(reopened.shutdown());
}

/// Verifies durable onboarding identity, workspace references, and human input.
///
/// # Parameters
/// - `database`: Temporary SQLite database used by the product process.
/// - `session`: Session identity created by the TUI agent composer.
fn assert_durable_session(database: &std::path::Path, session: EntityId) {
    let session_store = SqliteSessionStore::open(database).expect("open session store");
    let saved_session = session_store
        .load(session)
        .expect("read session")
        .expect("durable onboarding session");
    assert_eq!(saved_session.branches.len(), 1);
    assert_eq!(saved_session.branches[0].status, BranchStatus::Completed);
    let result_handles = &saved_session.branches[0]
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
        saved_session.branches[0].turns[0]
            .human
            .as_ref()
            .expect("human instruction")
            .text,
        "Onboard the current project."
    );
    drop(session_store);
}
