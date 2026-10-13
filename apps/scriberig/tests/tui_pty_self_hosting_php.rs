//! Verifies PHP self-extension proposal through a copied real checkout and PTY.

#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use scriberig_project::ProjectService;
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::{TuiPty, test_lock};

/// Drives onboarding and PHP candidate construction through the native product process.
#[test]
fn tui_self_hosts_php_candidate_without_changing_live_checkout() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_self_hosting();
    assert!(
        tui.read_until("project: unselected"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"start-agent\r");
    assert!(tui.read_until("Session:"), "{}", tui.diagnostic());
    let session = visible_session(&tui.screen());
    tui.send(b"Onboard the current project.\r");
    assert!(
        tui.read_until_for("The current project is onboarded", Duration::from_secs(120)),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.read_until("project: scriberig"), "{}", tui.diagnostic());
    let checkout = tui.fixture_path().join("scriberig");
    let scan_before =
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("baseline scanner");
    let root_manifest_before = fs::read(checkout.join("Cargo.toml")).expect("baseline manifest");
    let database = tui.fixture_path().join("scriberig.sqlite");
    let store = SqliteStore::open(&database).expect("open onboarding database");
    let project = store
        .list_projects()
        .expect("list projects")
        .into_iter()
        .find(|project| project.name == "scriberig")
        .expect("onboarded project");
    let base = ProjectService::new(Arc::new(
        SqliteStore::open(&database).expect("project store"),
    ))
    .workspace(project.id)
    .expect("active semantic workspace");
    tui.send(b"\x1b[B");
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"continue-agent\r");
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    tui.send(b"Add support for ingesting PHP source files.\r");
    assert!(
        tui.read_until_for("agent continuation [Completed]", Duration::from_secs(180),),
        "{}",
        tui.diagnostic()
    );
    assert_project_base(
        &checkout,
        &database,
        project.id,
        base.snapshot,
        &scan_before,
        &root_manifest_before,
    );
    let saved = load_session(&database, session);
    assert!(saved.branches.iter().any(|branch| {
        branch
            .purpose
            .as_deref()
            .is_some_and(|purpose| purpose.contains("determine minimal extension points"))
    }));
    assert_candidate_result(&saved);
    assert!(saved.branches.iter().any(|branch| {
        branch
            .purpose
            .as_deref()
            .is_some_and(|purpose| purpose.contains("implement PHP ingestion"))
    }));
    let _ = tui.shutdown_allow_failure();
}

/// Confirms candidate construction left the canonical snapshot and checkout unchanged.
///
/// # Parameters
/// - `checkout`: Copied project checkout used by the PTY process.
/// - `database`: Durable product database path.
/// - `project`: Onboarded project identity.
/// - `base_snapshot`: Exact snapshot captured after onboarding.
/// - `scanner`: Baseline scanner bytes.
/// - `manifest`: Baseline workspace manifest bytes.
fn assert_project_base(
    checkout: &std::path::Path,
    database: &std::path::Path,
    project: scriberig_core::ProjectId,
    base_snapshot: scriberig_core::SnapshotId,
    scanner: &[u8],
    manifest: &[u8],
) {
    let current = ProjectService::new(Arc::new(
        SqliteStore::open(database).expect("project store"),
    ))
    .workspace(project)
    .expect("project remains at base");
    assert_eq!(current.snapshot, base_snapshot);
    assert!(
        !checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
    assert_eq!(
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("scanner"),
        scanner
    );
    assert_eq!(
        fs::read(checkout.join("Cargo.toml")).expect("manifest"),
        manifest
    );
}

/// Loads the durable agent session created through the native TUI.
///
/// # Parameters
/// - `database`: Product database path.
/// - `session`: Exact session identity shown by the composer.
///
/// # Returns
/// Persisted session with completed branch results.
fn load_session(
    database: &std::path::Path,
    session: scriberig_core::EntityId,
) -> scriberig_session::Session {
    SqliteSessionStore::open(database)
        .expect("open session store")
        .load(session)
        .expect("read session")
        .expect("durable agent session")
}

/// Verifies the implementation branch returned its candidate through the root result.
///
/// # Parameters
/// - `session`: Persisted agent session to inspect.
fn assert_candidate_result(session: &scriberig_session::Session) {
    let root_result = session
        .branches
        .iter()
        .find(|branch| {
            branch.result.as_ref().is_some_and(|result| {
                result.summary.as_deref().is_some_and(|summary| {
                    summary.contains("validated PHP implementation candidate")
                })
            })
        })
        .and_then(|branch| branch.result.as_ref())
        .expect("root result contains candidate review summary");
    assert!(
        root_result
            .result_handles
            .iter()
            .any(|handle| handle.starts_with("candidate:"))
    );
}

/// Reads the session identity visible in the native agent composer.
///
/// # Parameters
/// - `screen`: Current rendered terminal screen.
///
/// # Returns
/// Session identity string.
fn visible_session(screen: &str) -> scriberig_core::EntityId {
    screen
        .split("Session:")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .expect("session visible in composer")
        .parse()
        .expect("session identity")
}
