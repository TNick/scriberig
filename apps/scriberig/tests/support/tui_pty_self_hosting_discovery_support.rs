//! Shared setup and assertions for self-hosting cancellation tests.

#[path = "../tui_pty_support.rs"]
#[allow(dead_code)]
pub(crate) mod tui_pty_support;

pub(crate) use tui_pty_support::test_lock;

use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_session::{BranchStatus, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::TuiPty;

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
pub(crate) struct OnboardingEvidence {
    /// Active native TUI process.
    pub(crate) tui: TuiPty,
    /// Durable agent session identity.
    pub(crate) session: EntityId,
    /// Copied Scriberig checkout.
    pub(crate) checkout: std::path::PathBuf,
    /// Fresh product database.
    pub(crate) database: std::path::PathBuf,
    /// Imported project identity.
    pub(crate) project: scriberig_core::ProjectId,
    /// Exact snapshot before PHP work.
    pub(crate) base: scriberig_core::SnapshotId,
    /// Original workspace manifest bytes.
    pub(crate) manifest: Vec<u8>,
    /// Original project scanner bytes.
    pub(crate) scanner: Vec<u8>,
}

/// Starts the real TUI and onboards its copied checkout through agent tools.
///
/// # Parameters
/// - `implementation`: Select the delayed nested branch phase.
///
/// # Returns
/// Durable onboarding state and exact source bytes for post-cancellation checks.
pub(crate) fn onboard_project(implementation: bool) -> OnboardingEvidence {
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
    assert!(
        tui.read_until("Agent composer · ent_"),
        "{}",
        tui.diagnostic()
    );
    let session = visible_session(&tui.screen());
    let database = tui.fixture_path().join("scriberig.sqlite");
    tui.send(b"Onboard the current project.\r");
    wait_for_onboarding(&database, session);
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

/// Waits until onboarding is durable and the project has been imported.
///
/// # Parameters
/// - `database`: Product database owned by the PTY fixture.
/// - `session`: Exact session identity to inspect.
pub(crate) fn wait_for_onboarding(database: &std::path::Path, session: EntityId) {
    let store = SqliteSessionStore::open(database).expect("session store");
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let saved = match store.load(session) {
            Ok(saved) => saved,
            Err(error) if error.contains("database is locked") => {
                assert!(
                    Instant::now() < deadline,
                    "onboarding database stayed locked"
                );
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(error) => panic!("load session: {error}"),
        };
        let Some(saved) = saved else {
            assert!(
                Instant::now() < deadline,
                "onboarding session was not persisted"
            );
            thread::sleep(Duration::from_millis(50));
            continue;
        };
        if saved.branches.iter().any(|branch| {
            branch.status == BranchStatus::Completed
                && branch
                    .tool_invocations
                    .iter()
                    .any(|call| call.name == "project_import")
        }) {
            return;
        }
        let branches = saved
            .branches
            .iter()
            .map(|branch| (branch.title.as_str(), branch.status))
            .collect::<Vec<_>>();
        assert!(
            Instant::now() < deadline,
            "onboarding did not complete: {branches:?}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Submits the PHP request and waits for architecture discovery to enter its delay.
///
/// # Parameters
/// - `tui`: Onboarded native TUI process.
/// - `session`: Durable agent session identity.
/// - `database`: Durable product database.
pub(crate) fn submit_discovery(tui: &mut TuiPty, session: EntityId, database: &std::path::Path) {
    tui.send(b"\x1b");
    assert!(tui.read_until("press i to"), "{}", tui.diagnostic());
    tui.send(b"\x1b[B");
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.type_palette_query("continue-agent")
        .expect("palette query");
    tui.send(b"\r");
    assert!(tui.read_until("Write a message..."), "{}", tui.diagnostic());
    tui.send(b"i");
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    tui.send(b"Add support for ingesting PHP source files.\r");
    wait_for_branch(database, session, "determine minimal extension points");
    assert_provider_gate_waiting(tui);
}

/// Submits PHP work and waits until its implementation child enters the delayed phase.
///
/// # Parameters
/// - `tui`: Onboarded native TUI process.
/// - `session`: Durable agent session identity.
/// - `database`: Durable product database.
pub(crate) fn submit_implementation(
    tui: &mut TuiPty,
    session: EntityId,
    database: &std::path::Path,
) {
    tui.send(b"\x1b");
    assert!(tui.read_until("press i to"), "{}", tui.diagnostic());
    tui.send(b"\x1b[B");
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.type_palette_query("continue-agent")
        .expect("palette query");
    tui.send(b"\r");
    assert!(tui.read_until("Write a message..."), "{}", tui.diagnostic());
    tui.send(b"i");
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    tui.send(b"Add support for ingesting PHP source files.\r");
    wait_for_branch(database, session, "implement PHP ingestion");
    assert_provider_gate_waiting(tui);
}

/// Verifies the deterministic provider reached the explicit cancellation barrier.
///
/// # Parameters
/// - `tui`: Active native TUI process owning the test gate path.
pub(crate) fn assert_provider_gate_waiting(tui: &TuiPty) {
    let release = tui.fixture_path().join("agent-provider.release");
    assert!(
        tui.fixture_path().join("agent-provider.waiting").is_file(),
        "provider did not reach its cancellation barrier"
    );
    assert!(!release.exists(), "provider release marker already exists");
}

/// Cancels the active agent from the shared operations view.
///
/// # Parameters
/// - `tui`: Active TUI process.
pub(crate) fn cancel_active_agent(tui: &mut TuiPty) {
    tui.send(b"\x1b");
    assert!(tui.read_until("press i to"), "{}", tui.diagnostic());
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"operations");
    assert!(tui.read_until("> View operations"), "{}", tui.diagnostic());
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
    tui.release_agent_provider();
}

/// Waits for one delayed child branch to start before requesting cancellation.
///
/// # Parameters
/// - `database`: Durable product database.
/// - `session`: Agent session identity.
/// - `purpose`: Unique substring of the child purpose.
pub(crate) fn wait_for_branch(database: &std::path::Path, session: EntityId, purpose: &str) {
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
pub(crate) fn assert_project_and_checkout_unchanged(evidence: &OnboardingEvidence) {
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
pub(crate) fn wait_for_cancellation(
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
        let branches = saved
            .branches
            .iter()
            .map(|branch| {
                (
                    branch.title.as_str(),
                    branch.status,
                    branch
                        .result
                        .as_ref()
                        .and_then(|result| result.result_kind.as_deref()),
                )
            })
            .collect::<Vec<_>>();
        assert!(
            Instant::now() < deadline,
            "cancelled result was not persisted: {branches:?}"
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
pub(crate) fn visible_session(screen: &str) -> EntityId {
    screen
        .split("Agent composer · ")
        .nth(1)
        .and_then(|text| text.split_whitespace().next())
        .expect("session displayed")
        .parse()
        .expect("session identity")
}
