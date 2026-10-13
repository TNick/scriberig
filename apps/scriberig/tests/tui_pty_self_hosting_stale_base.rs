//! Verifies a reviewed PHP candidate cannot apply after its base advances.

#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use scriberig_project::{CandidateValidation, ProjectService};
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::TuiPty;

/// Verifies stale review preserves the newer project and blocks PHP effects.
#[test]
fn tui_rejects_php_candidate_after_canonical_base_advances() {
    let mut scenario = start_stale_scenario();
    let (advanced, readme, changed_readme) = advance_canonical_project(&scenario);
    attempt_stale_apply(&mut scenario.tui);
    assert_stale_effects(&scenario, advanced, &readme, &changed_readme);

    let fixture = scenario.tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove stale candidate fixture");
}

/// Retains the exact project and candidate evidence for the stale Apply gate.
struct StaleScenario {
    /// Active real native TUI process.
    tui: TuiPty,
    /// Copied Scriberig checkout used by the PTY process.
    checkout: std::path::PathBuf,
    /// Shared project identity.
    project: scriberig_core::ProjectId,
    /// Snapshot against which the PHP candidate was created.
    base: scriberig_core::SnapshotId,
    /// Validated PHP candidate created by the agent.
    candidate: String,
    /// Shared project store used to inspect durable state.
    store: Arc<SqliteStore>,
}

/// Onboards the fixture and creates a validated PHP candidate through the agent.
///
/// # Returns
/// Durable candidate scope and the active real TUI process.
fn start_stale_scenario() -> StaleScenario {
    let mut tui = TuiPty::start_self_hosting();
    assert!(
        tui.read_until("project: unselected"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"start-agent\r");
    assert!(tui.read_until("Enter submit"), "{}", tui.diagnostic());
    let session = visible_session(&tui.screen());
    tui.send(b"Onboard the current project.\r");
    assert!(
        tui.read_until_for("The current project is onboarded", Duration::from_secs(120)),
        "{}",
        tui.diagnostic()
    );
    let checkout = tui.fixture_path().join("scriberig");
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

    let database = tui.fixture_path().join("scriberig.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("project store"));
    let project_service = ProjectService::new(Arc::clone(&store));
    let project = store
        .list_projects()
        .expect("projects")
        .into_iter()
        .find(|project| project.name == "scriberig")
        .expect("onboarded project");
    let base = project_service
        .workspace(project.id)
        .expect("candidate base workspace")
        .snapshot;
    let candidate = candidate_from_session(&database, session);
    let php_candidate = project_service
        .candidate(&candidate)
        .expect("candidate lookup")
        .expect("PHP candidate exists");
    assert_eq!(php_candidate.base_snapshot, base);
    assert_eq!(php_candidate.validation, CandidateValidation::Passed);
    assert!(
        !checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );

    StaleScenario {
        tui,
        checkout,
        project: project.id,
        base,
        candidate,
        store,
    }
}

/// Advances the current snapshot through a refreshed external README change.
///
/// # Parameters
/// - `scenario`: Exact project and base identities under stale-candidate test.
///
/// # Returns
/// The advanced snapshot, changed README path, and exact new README bytes.
fn advance_canonical_project(
    scenario: &StaleScenario,
) -> (scriberig_core::SnapshotId, std::path::PathBuf, Vec<u8>) {
    let readme = scenario.checkout.join("README.md");
    let base_readme = fs::read(&readme).expect("base README");
    let marker = b"\nExternal refresh advances this project before stale Apply.\n";
    let mut changed_readme = base_readme.clone();
    changed_readme.extend_from_slice(marker);
    fs::write(&readme, &changed_readme).expect("external README update");
    let project_service = ProjectService::new(Arc::clone(&scenario.store));
    project_service
        .scan(Some("scriberig".to_owned()), &scenario.checkout)
        .expect("persist changed manifest");
    let advanced = project_service
        .import(scenario.project)
        .expect("refresh project from changed manifest")
        .snapshot;
    assert_ne!(
        advanced, scenario.base,
        "external refresh must advance canonical state"
    );
    (advanced, readme, changed_readme)
}

/// Attempts shared TUI Apply for the candidate after the project base advances.
///
/// # Parameters
/// - `tui`: Active native TUI with the agent candidate in its session.
fn attempt_stale_apply(tui: &mut TuiPty) {
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"review-candidate\r");
    assert!(tui.read_until("Change review"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("validation: Passed"),
        "{}",
        tui.screen()
    );
    tui.send(b"\r");
    assert!(tui.read_until("Enter confirm"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(
        tui.read_until_for("stale_candidate", Duration::from_secs(30)),
        "{}",
        tui.diagnostic()
    );
}

/// Verifies the stale Apply receipt and protects the newer project state.
///
/// # Parameters
/// - `scenario`: Candidate and project identities from the real PTY scenario.
/// - `advanced`: New authoritative snapshot that superseded the candidate base.
/// - `readme`: Externally changed README path.
/// - `changed_readme`: Exact externally refreshed README bytes.
fn assert_stale_effects(
    scenario: &StaleScenario,
    advanced: scriberig_core::SnapshotId,
    readme: &std::path::Path,
    changed_readme: &[u8],
) {
    let project_service = ProjectService::new(Arc::clone(&scenario.store));
    assert_eq!(
        project_service
            .workspace(scenario.project)
            .expect("newer workspace remains authoritative")
            .snapshot,
        advanced
    );
    assert_eq!(
        fs::read(readme).expect("external bytes survive"),
        changed_readme
    );
    assert!(
        !scenario
            .checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
    let receipt = scenario
        .store
        .project_effect(&scenario.candidate)
        .expect("effect receipt")
        .expect("stale attempt is durable");
    assert_eq!(receipt.state, "failed");
    assert_eq!(receipt.successor_snapshot, None);
    assert!(
        receipt
            .diagnostic
            .as_deref()
            .is_some_and(|diagnostic| diagnostic.contains("snapshot advanced"))
    );
}

/// Loads the candidate identity returned by the durable agent session.
///
/// # Parameters
/// - `database`: Durable product database path.
/// - `session`: Session that proposed the candidate.
///
/// # Returns
/// The exact project candidate identity retained by the session result.
fn candidate_from_session(database: &std::path::Path, session: scriberig_core::EntityId) -> String {
    SqliteSessionStore::open(database)
        .expect("session store")
        .load(session)
        .expect("load session")
        .expect("durable session")
        .branches
        .iter()
        .filter_map(|branch| branch.result.as_ref())
        .flat_map(|result| &result.result_handles)
        .find_map(|handle| handle.strip_prefix("candidate:").map(str::to_owned))
        .expect("candidate handle")
}

/// Reads the durable session identity shown in the agent composer.
///
/// # Parameters
/// - `screen`: Current native TUI screen.
///
/// # Returns
/// The session identity rendered by the composer.
fn visible_session(screen: &str) -> scriberig_core::EntityId {
    screen
        .split("Session:")
        .nth(1)
        .and_then(|text| text.split_whitespace().next())
        .expect("session visible in composer")
        .parse()
        .expect("session identity")
}
