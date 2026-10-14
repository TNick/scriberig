//! Verifies post-onboarding architecture branch cancellation is durable and isolated.

#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_session::{BranchStatus, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::{TuiPty, test_lock};

/// Cancels a read-only architecture child after onboarding a real copied checkout.
#[test]
fn tui_cancels_php_discovery_without_project_or_source_mutation() {
    let _lock = test_lock();
    let mut evidence = onboard_project(false);
    submit_discovery(&mut evidence.tui, evidence.session, &evidence.database);
    cancel_active_agent(&mut evidence.tui);
    let saved = wait_for_cancellation(&evidence.database, evidence.session);
    let discovery = saved
        .branches
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose.contains("determine minimal extension points"))
        })
        .expect("cancelled child branch");
    assert_eq!(discovery.status, BranchStatus::Abandoned);
    assert!(saved.branches.iter().any(|branch| {
        branch
            .result
            .as_ref()
            .is_some_and(|result| result.result_kind.as_deref() == Some("agent-cancelled"))
    }));
    assert_eq!(
        ProjectService::new(std::sync::Arc::new(
            SqliteStore::open(&evidence.database).expect("reopen project store"),
        ))
        .workspace(evidence.project)
        .expect("project remains active")
        .snapshot,
        evidence.base
    );
    assert_eq!(
        fs::read(evidence.checkout.join("Cargo.toml")).expect("manifest"),
        evidence.manifest
    );
    assert_eq!(
        fs::read(
            evidence
                .checkout
                .join("crates/scriberig-project/src/scan.rs")
        )
        .expect("scanner"),
        evidence.scanner
    );
    assert!(
        !evidence
            .checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
    let fixture = evidence.tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove cancellation fixture");
}

/// Cancels implementation after bounded B1 discovery and before candidate creation.
#[test]
fn tui_cancels_php_implementation_without_project_or_source_mutation() {
    let _lock = test_lock();
    let mut evidence = onboard_project(true);
    submit_implementation(&mut evidence.tui, evidence.session, &evidence.database);
    cancel_active_agent(&mut evidence.tui);
    let saved = wait_for_cancellation(&evidence.database, evidence.session);
    let implementation = saved
        .branches
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose.contains("implement PHP ingestion"))
        })
        .expect("cancelled implementation branch");
    assert_eq!(implementation.status, BranchStatus::Abandoned);
    assert_eq!(implementation.tool_invocations.len(), 0);
    assert_project_and_checkout_unchanged(&evidence);
    let fixture = evidence.tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove cancellation fixture");
}

/// Captures the onboarding identities and unchanged checkout bytes for cancellation assertions.
///
/// Public members: None.
///
/// Private members
/// - `tui`: Active TUI process.
/// - `session`: Durable agent session identity.
/// - `checkout`: Copied Scriberig checkout.
/// - `database`: Fresh product database.
/// - `project`: Imported project identity.
/// - `base`: Exact snapshot before PHP work.
/// - `manifest`: Original workspace manifest bytes.
/// - `scanner`: Original project scanner bytes.
struct OnboardingEvidence {
    /// Active native TUI process.
    tui: TuiPty,
    /// Durable agent session identity.
    session: EntityId,
    /// Copied Scriberig checkout.
    checkout: std::path::PathBuf,
    /// Fresh product database.
    database: std::path::PathBuf,
    /// Imported project identity.
    project: scriberig_core::ProjectId,
    /// Exact snapshot before PHP work.
    base: scriberig_core::SnapshotId,
    /// Original workspace manifest bytes.
    manifest: Vec<u8>,
    /// Original project scanner bytes.
    scanner: Vec<u8>,
}

/// Starts the real TUI and onboards its copied checkout through agent tools.
///
/// # Parameters
/// - `implementation`: Select the delayed nested branch phase.
///
/// # Returns
/// Durable onboarding state and exact source bytes for post-cancellation checks.
fn onboard_project(implementation: bool) -> OnboardingEvidence {
    let mut tui = if implementation {
        TuiPty::start_self_hosting_implementation_cancellation()
    } else {
        TuiPty::start_self_hosting_discovery_cancellation()
    };
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
    let session = visible_session(&tui.screen());
    tui.send(b"Onboard the current project.\r");
    assert!(
        tui.read_until_for("The current project is onboarded", Duration::from_secs(120)),
        "{}",
        tui.diagnostic()
    );
    assert!(tui.read_until("project: scriberig"), "{}", tui.diagnostic());
    let checkout = tui.fixture_path().join("scriberig");
    let manifest = fs::read(checkout.join("Cargo.toml")).expect("workspace manifest");
    let scanner =
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("project scanner");
    let database = tui.fixture_path().join("scriberig.sqlite");
    let store = SqliteStore::open(&database).expect("project store");
    let project = store
        .list_projects()
        .expect("projects")
        .into_iter()
        .find(|record| record.name == "scriberig")
        .expect("onboarded project")
        .id;
    let base = ProjectService::new(std::sync::Arc::new(store))
        .workspace(project)
        .expect("base workspace")
        .snapshot;
    OnboardingEvidence {
        tui,
        session,
        checkout,
        database,
        project,
        base,
        manifest,
        scanner,
    }
}

/// Submits the PHP request and waits for architecture discovery to enter its delay.
///
/// # Parameters
/// - `tui`: Onboarded native TUI process.
/// - `session`: Durable agent session identity.
/// - `database`: Durable product database.
fn submit_discovery(tui: &mut TuiPty, session: EntityId, database: &std::path::Path) {
    tui.send(b"\x1b[B");
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"continue-agent\r");
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    tui.send(b"Add support for ingesting PHP source files.\r");
    wait_for_discovery(database, session);
}

/// Submits PHP work and waits until its implementation child enters the delayed phase.
///
/// # Parameters
/// - `tui`: Onboarded native TUI process.
/// - `session`: Durable agent session identity.
/// - `database`: Durable product database.
fn submit_implementation(tui: &mut TuiPty, session: EntityId, database: &std::path::Path) {
    tui.send(b"\x1b[B");
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"continue-agent\r");
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    tui.send(b"Add support for ingesting PHP source files.\r");
    wait_for_branch(database, session, "implement PHP ingestion");
}

/// Cancels the active agent from the shared operations view.
///
/// # Parameters
/// - `tui`: Active TUI process.
fn cancel_active_agent(tui: &mut TuiPty) {
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"operations");
    assert!(tui.read_until("View operations"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(
        tui.read_until_for("Running", Duration::from_secs(10)),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"x");
    assert!(
        tui.read_until_for("Cancelled", Duration::from_secs(10)),
        "{}",
        tui.diagnostic()
    );
}

/// Waits for the running discovery branch before requesting cancellation.
///
/// # Parameters
/// - `database`: Durable product database.
/// - `session`: Agent session identity.
fn wait_for_discovery(database: &std::path::Path, session: EntityId) {
    wait_for_branch(database, session, "determine minimal extension points");
}

/// Waits for one delayed child branch to start before requesting cancellation.
///
/// # Parameters
/// - `database`: Durable product database.
/// - `session`: Agent session identity.
/// - `purpose`: Unique substring of the child purpose.
fn wait_for_branch(database: &std::path::Path, session: EntityId, purpose: &str) {
    let store = SqliteSessionStore::open(database).expect("session store");
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let saved = store
            .load(session)
            .expect("load session")
            .expect("durable session");
        if saved.branches.iter().any(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|branch_purpose| branch_purpose.contains(purpose))
                && branch.tool_invocations.is_empty()
        }) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "child branch {purpose:?} did not start"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Verifies cancellation leaves canonical project state and copied source bytes unchanged.
///
/// # Parameters
/// - `evidence`: Onboarded project identities and exact source bytes.
fn assert_project_and_checkout_unchanged(evidence: &OnboardingEvidence) {
    assert_eq!(
        ProjectService::new(std::sync::Arc::new(
            SqliteStore::open(&evidence.database).expect("reopen project store"),
        ))
        .workspace(evidence.project)
        .expect("project remains active")
        .snapshot,
        evidence.base
    );
    assert_eq!(
        fs::read(evidence.checkout.join("Cargo.toml")).expect("manifest"),
        evidence.manifest
    );
    assert_eq!(
        fs::read(
            evidence
                .checkout
                .join("crates/scriberig-project/src/scan.rs")
        )
        .expect("scanner"),
        evidence.scanner
    );
    assert!(
        !evidence
            .checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
}

/// Waits for the session store to persist an agent cancellation result.
///
/// # Parameters
/// - `database`: Durable product database.
/// - `session`: Agent session identity.
///
/// # Returns
/// Session with a durable cancelled branch result.
fn wait_for_cancellation(
    database: &std::path::Path,
    session: EntityId,
) -> scriberig_session::Session {
    let store = SqliteSessionStore::open(database).expect("session store");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let saved = store
            .load(session)
            .expect("load session")
            .expect("durable session");
        if saved.branches.iter().any(|branch| {
            branch
                .result
                .as_ref()
                .is_some_and(|result| result.result_kind.as_deref() == Some("agent-cancelled"))
        }) {
            return saved;
        }
        assert!(
            Instant::now() < deadline,
            "cancelled result was not persisted"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Parses the durable session identity displayed by the agent composer.
///
/// # Parameters
/// - `screen`: Current native TUI screen text.
///
/// # Returns
/// Durable agent session identity.
fn visible_session(screen: &str) -> EntityId {
    screen
        .split("Session:")
        .nth(1)
        .and_then(|text| text.split_whitespace().next())
        .expect("session displayed")
        .parse()
        .expect("session identity")
}
