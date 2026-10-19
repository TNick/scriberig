//! Verifies failed PHP candidate validation remains reviewable and unapplied.

#[path = "tui_pty_support/onboarding_wait.rs"]
mod onboarding_wait;
#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use onboarding_wait::{wait_for_branch_completion, wait_for_onboarding};
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
    let mut scenario = onboard_failure_scenario(false);
    submit_php_continuation(&mut scenario.tui, Duration::from_secs(300));
    let candidate = assert_failed_candidate(&scenario.database, scenario.session);
    assert_project_unchanged(&scenario, &candidate);
    review_failed_candidate(&mut scenario.tui);
    let fixture = scenario.tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove validation failure fixture");
}

/// Verifies the agent repairs an invalid candidate through a second normal proposal.
#[test]
fn tui_agent_repairs_validation_failure_with_a_new_project_candidate() {
    let _lock = test_lock();
    let mut scenario = onboard_failure_scenario(true);
    submit_php_continuation(&mut scenario.tui, Duration::from_secs(900));
    let saved = SqliteSessionStore::open(&scenario.database)
        .expect("session store")
        .load(scenario.session)
        .expect("load session")
        .expect("durable session");
    let implementation = saved
        .branches
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose.contains("implement PHP ingestion"))
        })
        .expect("implementation branch");
    let proposals = implementation
        .tool_invocations
        .iter()
        .filter(|invocation| invocation.name == "project_propose")
        .collect::<Vec<_>>();
    assert_eq!(
        proposals.len(),
        2,
        "repair must use a second normal proposal"
    );
    let candidates = proposals
        .iter()
        .map(|invocation| {
            let input: serde_json::Value =
                serde_json::from_slice(&invocation.input).expect("proposal input");
            input["effects"]
                .as_array()
                .expect("candidate effects")
                .iter()
                .find(|effect| effect["path"] == "crates/domains/scriberig-domain-php/src/lib.rs")
                .expect("PHP adapter root effect")["content"]
                .as_str()
                .expect("PHP adapter root content")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert!(candidates[0].contains("deterministic validation failure"));
    assert!(!candidates[1].contains("deterministic validation failure"));
    let repaired_result = implementation
        .tool_results
        .iter()
        .find(|result| result.invocation == proposals[1].id)
        .expect("second proposal result");
    let repaired_output: serde_json::Value =
        serde_json::from_slice(&repaired_result.output).expect("second proposal output");
    let repaired_id = repaired_output["handles"]
        .as_array()
        .expect("candidate handles")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .find_map(|handle| handle.strip_prefix("candidate:"))
        .expect("repaired candidate handle");
    let repaired = ProjectService::new(Arc::new(
        SqliteStore::open(&scenario.database).expect("project store"),
    ))
    .candidate(repaired_id)
    .expect("load repaired candidate")
    .expect("repaired candidate exists");
    assert_eq!(repaired.validation, CandidateValidation::Passed);
    assert_project_unchanged(&scenario, repaired_id);
    let continuation = saved
        .branches
        .iter()
        .find(|branch| {
            branch.result.as_ref().is_some_and(|result| {
                result.summary.as_deref().is_some_and(|summary| {
                    summary.contains("validated PHP implementation candidate")
                })
            })
        })
        .expect("successful repair continuation");
    assert!(
        continuation
            .result
            .as_ref()
            .expect("continuation result")
            .result_handles
            .iter()
            .any(|handle| handle == &format!("candidate:{repaired_id}"))
    );
    let fixture = scenario.tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove validation repair fixture");
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
/// - `repair`: Selects the invalid-first-proposal repair provider scenario.
///
/// Durable session, exact base project scope, and original live source bytes.
/// # Returns
/// The operation result.
fn onboard_failure_scenario(repair: bool) -> FailureScenario {
    let mut tui = if repair {
        TuiPty::start_self_hosting_validation_repair()
    } else {
        TuiPty::start_self_hosting_validation_failure()
    };
    assert!(tui.read_until("no active project"), "{}", tui.diagnostic());
    tui.send(b"Onboard the current project.\r");
    let session = visible_session(&tui);
    wait_for_onboarding(&mut tui, session);
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
/// - `tui`: Active TUI displaying the onboarded project.
/// - `timeout`: Maximum wait for the agent continuation.
fn submit_php_continuation(tui: &mut TuiPty, timeout: Duration) {
    tui.send(b"Add support for ingesting PHP source files.\r");
    let session = visible_session(tui);
    wait_for_branch_completion(tui, session, "agent continuation", timeout);
}

/// Reads and verifies the durable candidate created by the failed validation run.
///
/// - `database`: Product database containing project and candidate state.
/// - `session`: Exact durable session that proposed the candidate.
///
/// The failed candidate identity.
/// # Returns
/// The operation result.
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
/// - `tui`: Active TUI with the failed candidate in its durable session result.
fn review_failed_candidate(tui: &mut TuiPty) {
    tui.send(b"/changes\r");
    assert!(tui.read_until("Change review"), "{}", tui.diagnostic());
    assert!(
        tui.screen().contains("validation: Failed"),
        "{}",
        tui.screen()
    );
}

/// Reads the durable session identity shown by the agent composer.
///
/// - `tui`: Active native TUI process.
///
/// The session identity rendered by the composer.
/// # Returns
/// The operation result.
fn visible_session(tui: &TuiPty) -> EntityId {
    tui.stored_session_id()
}
