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
/// - `implementation`: Select the delayed nested branch phase.
///
/// Durable onboarding state and exact source bytes for post-cancellation checks.
/// # Returns
/// The operation result.
pub(crate) fn onboard_project(implementation: bool) -> OnboardingEvidence {
    let mut tui = if implementation {
        TuiPty::start_self_hosting_implementation_cancellation()
    } else {
        TuiPty::start_self_hosting_discovery_cancellation()
    };
    assert!(tui.read_until("no active project"), "{}", tui.diagnostic());
    let database = tui.fixture_path().join("scriberig.sqlite");
    tui.send(b"Onboard the current project.\r");
    let session = visible_session(&tui);
    wait_for_onboarding(&database, session);
    assert!(
        tui.read_until_next("Enter send"),
        "TUI did not return to its conversation composer: {}",
        tui.diagnostic()
    );
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
        let root_completed = saved
            .branches
            .iter()
            .any(|branch| branch.title == "agent root" && branch.status == BranchStatus::Completed);
        let import_completed = saved.branches.iter().any(|branch| {
            branch.status == BranchStatus::Completed
                && branch
                    .tool_invocations
                    .iter()
                    .any(|call| call.name == "project_import")
        });
        if root_completed && import_completed {
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
/// - `tui`: Onboarded native TUI process.
/// - `session`: Durable agent session identity.
/// - `database`: Durable product database.
pub(crate) fn submit_discovery(tui: &mut TuiPty, session: EntityId, database: &std::path::Path) {
    tui.send(b"Add support for ingesting PHP source files.\r");
    wait_for_branch(database, session, "determine minimal extension points");
    assert_provider_gate_waiting(tui);
}

/// Submits PHP work and waits until its implementation child enters the delayed phase.
///
/// - `tui`: Onboarded native TUI process.
/// - `session`: Durable agent session identity.
/// - `database`: Durable product database.
pub(crate) fn submit_implementation(
    tui: &mut TuiPty,
    session: EntityId,
    database: &std::path::Path,
) {
    tui.send(b"Add support for ingesting PHP source files.\r");
    wait_for_branch(database, session, "implement PHP ingestion");
    assert_provider_gate_waiting(tui);
}

/// Verifies the deterministic provider reached the explicit cancellation barrier.
///
/// - `tui`: Active native TUI process owning the test gate path.
pub(crate) fn assert_provider_gate_waiting(tui: &TuiPty) {
    let release = tui.fixture_path().join("agent-provider.release");
    let waiting = tui.fixture_path().join("agent-provider.waiting");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !waiting.is_file() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(25));
    }
    assert!(
        waiting.is_file(),
        "provider did not reach its cancellation barrier"
    );
    assert!(!release.exists(), "provider release marker already exists");
}

/// Cancels the active agent from the shared operations view.
///
/// - `tui`: Active TUI process.
pub(crate) fn cancel_active_agent(tui: &mut TuiPty) {
    tui.send(b"/operations\r");
    assert!(tui.read_until("Operations"), "{}", tui.diagnostic());
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
        }) {
            return;
        }
        let branches = saved
            .branches
            .iter()
            .map(|branch| {
                (
                    branch.title.as_str(),
                    branch.purpose.as_deref(),
                    branch.status,
                    branch
                        .result
                        .as_ref()
                        .and_then(|result| result.summary.as_deref()),
                )
            })
            .collect::<Vec<_>>();
        assert!(
            Instant::now() < deadline,
            "child branch {purpose:?} did not start: {branches:?}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Verifies cancellation leaves canonical project state and copied source bytes unchanged.
///
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
/// - `database`: Durable product database.
/// - `session`: Agent session identity.
///
/// Session with a durable cancelled branch result.
/// # Returns
/// The operation result.
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
/// - `tui`: Active native TUI process.
///
/// Durable agent session identity.
/// # Returns
/// The operation result.
pub(crate) fn visible_session(tui: &TuiPty) -> EntityId {
    tui.stored_session_id()
}
