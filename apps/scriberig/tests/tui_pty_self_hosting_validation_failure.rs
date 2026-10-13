//! Verifies failed PHP candidate validation remains reviewable and unapplied.

#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_project::{CandidateValidation, ProjectService};
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::{TuiPty, test_lock};

/// Drives an intentionally invalid PHP candidate through the native TUI.
#[test]
fn tui_validation_failure_keeps_candidate_pending_review_and_project_at_base() {
    let _lock = test_lock();
    let mut scenario = onboard_failure_scenario();
    submit_php_continuation(&mut scenario.tui);
    let candidate = assert_failed_candidate(&scenario.database, scenario.session);
    assert_project_unchanged(&scenario, &candidate);
    review_failed_candidate(&mut scenario.tui);
    let fixture = scenario.tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove validation failure fixture");
}

/// Identifies durable onboarding state used by the failed-validation scenario.
///
/// Public members: None.
///
/// Private members
/// - `tui`: Active real native TUI process.
/// - `session`: Durable agent session identity.
/// - `checkout`: Copied Scriberig source checkout.
/// - `database`: Fresh product database path.
/// - `project`: Imported semantic project identity.
/// - `base`: Exact active project snapshot before candidate creation.
/// - `manifest`: Original Cargo workspace manifest bytes.
/// - `scanner`: Original project scanner source bytes.
struct FailureScenario {
    /// Active real native TUI process.
    tui: TuiPty,
    /// Durable agent session identity.
    session: EntityId,
    /// Copied Scriberig source checkout.
    checkout: PathBuf,
    /// Fresh product database path.
    database: PathBuf,
    /// Imported semantic project identity.
    project: ProjectId,
    /// Exact active project snapshot before candidate creation.
    base: SnapshotId,
    /// Original Cargo workspace manifest bytes.
    manifest: Vec<u8>,
    /// Original project scanner source bytes.
    scanner: Vec<u8>,
}

/// Starts the failure scenario and onboards its copied checkout through the TUI.
///
/// # Returns
/// Durable session, exact base project scope, and original live source bytes.
fn onboard_failure_scenario() -> FailureScenario {
    let mut tui = TuiPty::start_self_hosting_validation_failure();
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
    let manifest = fs::read(checkout.join("Cargo.toml")).expect("base manifest");
    let scanner =
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("base scanner");
    let database = tui.fixture_path().join("scriberig.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("onboarded store"));
    let project = store
        .list_projects()
        .expect("projects")
        .into_iter()
        .find(|record| record.name == "scriberig")
        .expect("onboarded project")
        .id;
    let base = ProjectService::new(store)
        .workspace(project)
        .expect("base semantic workspace")
        .snapshot;
    FailureScenario {
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

/// Continues the onboarded session with the exact PHP extension request.
///
/// # Parameters
/// - `tui`: Active TUI displaying the onboarded project.
fn submit_php_continuation(tui: &mut TuiPty) {
    tui.send(b"\x1b[B");
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"continue-agent\r");
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    tui.send(b"Add support for ingesting PHP source files.\r");
    assert!(
        tui.read_until_for("agent continuation [Completed]", Duration::from_secs(300)),
        "{}",
        tui.diagnostic()
    );
}

/// Reads and verifies the durable candidate created by the failed validation run.
///
/// # Parameters
/// - `database`: Product database containing project and candidate state.
/// - `session`: Exact durable session that proposed the candidate.
///
/// # Returns
/// The failed candidate identity.
fn assert_failed_candidate(database: &std::path::Path, session: EntityId) -> String {
    let saved = SqliteSessionStore::open(database)
        .expect("session store")
        .load(session)
        .expect("load session")
        .expect("durable session");
    let candidate_id = saved
        .branches
        .iter()
        .filter_map(|branch| branch.result.as_ref())
        .flat_map(|result| &result.result_handles)
        .find_map(|handle| handle.strip_prefix("candidate:"))
        .expect("failed candidate handle")
        .to_owned();
    let candidate = ProjectService::new(Arc::new(SqliteStore::open(database).expect("store")))
        .candidate(&candidate_id)
        .expect("candidate lookup")
        .expect("durable failed candidate");
    assert!(
        matches!(&candidate.validation, CandidateValidation::Failed { diagnostics } if !diagnostics.is_empty()),
        "candidate validation state: {:?}",
        candidate.validation
    );
    candidate_id
}

/// Verifies failed validation left the canonical project and checkout at base.
///
/// # Parameters
/// - `scenario`: Captured project identities and exact original source bytes.
/// - `candidate`: Failed candidate identity retained for external review.
fn assert_project_unchanged(scenario: &FailureScenario, candidate: &str) {
    let store = Arc::new(SqliteStore::open(&scenario.database).expect("project store"));
    assert_eq!(
        ProjectService::new(store)
            .workspace(scenario.project)
            .expect("project remains at base")
            .snapshot,
        scenario.base,
        "failed candidate {candidate} advanced the canonical project"
    );
    assert!(
        !scenario
            .checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
    assert_eq!(
        fs::read(scenario.checkout.join("Cargo.toml")).expect("manifest"),
        scenario.manifest
    );
    assert_eq!(
        fs::read(
            scenario
                .checkout
                .join("crates/scriberig-project/src/scan.rs")
        )
        .expect("scanner"),
        scenario.scanner
    );
}

/// Opens the shared review surface and verifies failed validation is visible.
///
/// # Parameters
/// - `tui`: Active TUI with the failed candidate in its durable session result.
fn review_failed_candidate(tui: &mut TuiPty) {
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"review-candidate\r");
    assert!(tui.read_until("Change review"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("validation: Failed"),
        "{}",
        tui.screen()
    );
}

/// Reads the durable session identity shown by the agent composer.
///
/// # Parameters
/// - `screen`: Current native TUI screen.
///
/// # Returns
/// The session identity rendered by the composer.
fn visible_session(screen: &str) -> EntityId {
    screen
        .split("Session:")
        .nth(1)
        .and_then(|text| text.split_whitespace().next())
        .expect("session visible in composer")
        .parse()
        .expect("session identity")
}
