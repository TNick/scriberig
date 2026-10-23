//! Verifies a reviewed PHP candidate cannot apply after its base advances.

#[path = "tui_pty_support/onboarding_wait.rs"]
mod onboarding_wait;
#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use onboarding_wait::{wait_for_branch_completion, wait_for_onboarding};
use scriberig_project::{CandidateValidation, ProjectService};
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::{TuiPty, candidate_review};

/// Verifies stale review preserves the newer project and blocks PHP effects.
#[test]
fn tui_rejects_php_candidate_after_canonical_base_advances() {
    let mut scenario = start_stale_scenario();
    let (advanced, readme, changed_readme) = advance_canonical_project(&scenario);
    attempt_stale_apply(&mut scenario.tui);
    assert_stale_effects(&scenario, advanced, &readme, &changed_readme);

    drop(scenario.store);
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
/// Durable candidate scope and the active real TUI process.
/// # Returns
/// The operation result.
fn start_stale_scenario() -> StaleScenario {
    let mut tui = TuiPty::start_self_hosting();
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    tui.send(b"Onboard the current project.\r");
    let session = visible_session(&tui);
    wait_for_onboarding(&mut tui, session);
    let checkout = tui.fixture_path().join("scriberig");
    tui.send(b"Add support for ingesting PHP source files.\r");
    wait_for_branch_completion(
        &tui,
        session,
        "agent continuation",
        Duration::from_secs(600),
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
/// - `scenario`: Exact project and base identities under stale-candidate test.
///
/// The advanced snapshot, changed README path, and exact new README bytes.
/// # Returns
/// The operation result.
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
/// - `tui`: Active native TUI with the agent candidate in its session.
fn attempt_stale_apply(tui: &mut TuiPty) {
    candidate_review::open_candidate_review(tui, "Validation: Passed");
    assert!(
        tui.screen().contains("Validation: Passed"),
        "{}",
        tui.screen()
    );
    candidate_review::open_apply_confirmation(tui);
    candidate_review::confirm_candidate_apply(tui);
    assert!(
        tui.read_until_for("stale_candidate", Duration::from_secs(30)),
        "{}",
        tui.diagnostic()
    );
    candidate_review::assert_confirmation_closed(tui);
}

/// Verifies the stale Apply receipt and protects the newer project state.
///
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
/// - `database`: Durable product database path.
/// - `session`: Session that proposed the candidate.
///
/// The exact project candidate identity retained by the session result.
/// # Returns
/// The operation result.
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
/// - `tui`: Active native TUI process.
///
/// The session identity rendered by the composer.
/// # Returns
/// The operation result.
fn visible_session(tui: &TuiPty) -> scriberig_core::EntityId {
    tui.stored_session_id()
}
