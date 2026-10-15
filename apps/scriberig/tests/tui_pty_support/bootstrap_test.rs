//! Verifies real-process agent onboarding and durable workspace refresh.

use super::tui_pty_support::{TuiPty, test_lock};
use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_session::{BranchStatus, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;

/// Verifies failed import after manifest review leaves the project inactive and source unchanged.
#[test]
fn tui_bootstrap_import_failure_does_not_activate_partial_project() {
    let _lock = super::tui_pty_support::test_lock();
    let mut tui = super::tui_pty_support::TuiPty::start_bootstrap_import_failure();
    assert!(
        tui.read_until("project: unselected"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"start-agent\r");
    assert!(tui.read_until("Session:"), "{}", tui.diagnostic());
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    let session = tui
        .screen()
        .split("Session:")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .expect("visible session")
        .parse::<EntityId>()
        .expect("session identity");
    let checkout = tui.fixture_path().join("scriberig");
    let manifest = std::fs::read(checkout.join("Cargo.toml")).expect("baseline manifest");
    let source = std::fs::read(checkout.join("crates/scriberig-project/src/scan.rs"))
        .expect("baseline scanner");

    tui.send(b"Onboard the current project.\r");
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"operations");
    assert!(tui.read_until("View operations"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(
        tui.read_until_for("Failed", std::time::Duration::from_secs(60)),
        "{}",
        tui.diagnostic()
    );

    assert!(tui.screen().contains("project: unselected"));
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

    let saved = SqliteSessionStore::open(&database)
        .expect("session store")
        .load(session)
        .expect("durable session")
        .expect("session exists");
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
    assert!(
        tui.read_until("project: unselected"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"start-agent\r");
    assert!(tui.read_until("Session:"), "{}", tui.diagnostic());
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    let session = tui
        .screen()
        .split("Session:")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .expect("session visible in composer")
        .parse::<EntityId>()
        .expect("session identity");
    tui.send(b"Onboard the current project.\r");
    let onboarded = tui.read_until("The current project is onboarded");
    let saved = SqliteSessionStore::open(tui.fixture_path().join("scriberig.sqlite"))
        .expect("open diagnostic session store")
        .load(session)
        .expect("read diagnostic session");
    assert!(onboarded, "{}; saved session: {saved:?}", tui.diagnostic());
    assert!(tui.read_until("project: scriberig"), "{}", tui.diagnostic());
    tui.send(b"\x1b[B");
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"continue-agent");
    assert!(
        tui.read_until("Search: continue-agent"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains(&session.to_string()),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"Discover Scriberig architecture semantically.");
    assert!(
        tui.read_until("Discover Scriberig architecture semantically."),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    let discovered = tui.read_until_for(
        "agent continuation [Completed]",
        std::time::Duration::from_secs(120),
    );
    let saved = SqliteSessionStore::open(tui.fixture_path().join("scriberig.sqlite"))
        .expect("open discovery diagnostic session store")
        .load(session)
        .expect("read discovery diagnostic session");
    assert!(discovered, "{}; saved session: {saved:?}", tui.diagnostic());

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
    assert_durable_discovery(&database, session, project.id, workspace.snapshot);
    drop(store);
    let fixture = tui.quit_retain_fixture();

    let mut reopened = TuiPty::restart_bootstrap(fixture.clone());
    assert!(
        reopened.read_until("scriberig @ snp_"),
        "{}",
        reopened.diagnostic()
    );
    assert_durable_discovery(&database, session, project.id, workspace.snapshot);
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
    assert!(saved_session.branches.len() >= 2);
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

/// Verifies semantic child discovery is durable and exact-snapshot scoped.
///
/// # Parameters
/// - `database`: Temporary SQLite database used by the product process.
/// - `session`: Session receiving parent and child discovery branches.
/// - `project`: Onboarded project identity.
/// - `snapshot`: Exact immutable workspace snapshot.
fn assert_durable_discovery(
    database: &std::path::Path,
    session: EntityId,
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) {
    let session_store = SqliteSessionStore::open(database).expect("open session store");
    let saved = session_store
        .load(session)
        .expect("read session")
        .expect("durable discovery session");
    let child = saved
        .branches
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose.contains("determine minimal extension points"))
        })
        .expect("nested architecture child branch");
    assert_eq!(child.status, BranchStatus::Completed);
    assert!(child.parent.is_some());
    let result = child.result.as_ref().expect("durable child BranchResult");
    assert!(
        result
            .summary
            .as_deref()
            .unwrap_or_default()
            .contains("shared code domain")
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle == &format!("project:{project}"))
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle == &format!("snapshot:{snapshot}"))
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle == "path:src/lib.rs")
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle.starts_with("source_digest:"))
    );
}
