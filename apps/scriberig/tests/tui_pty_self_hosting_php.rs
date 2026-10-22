//! Verifies PHP self-extension proposal through a copied real checkout and PTY.

#[path = "tui_pty_support/self_hosting_php_assertions.rs"]
mod assertions;
#[path = "tui_pty_support/branch_assertions.rs"]
mod branch_assertions;
#[path = "tui_pty_support/onboarding_wait.rs"]
mod onboarding_wait;
#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use assertions::{
    assert_applied_history_transition, assert_applied_php, assert_candidate_conflicts_are_visible,
    assert_durable_candidate_effect, assert_materialized_php_fixture,
    assert_unrelated_source_unchanged,
};
use branch_assertions::assert_nested_branch_results;
use onboarding_wait::{visible_session, wait_for_branch_completion, wait_for_onboarding};
use scriberig_project::ProjectService;
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::{TuiPty, candidate_review, test_lock};

/// Drives onboarding and PHP candidate construction through the native product process.
#[test]
fn tui_self_hosts_php_candidate_without_changing_live_checkout() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_self_hosting();
    let checkout = tui.fixture_path().join("scriberig");
    assertions::assert_tracked_serena_configuration(&checkout);
    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send_slow(b"/project\r");
    assert!(
        tui.read_until("No active project is selected."),
        "{}",
        tui.diagnostic()
    );
    tui.send_slow(b"Onboard the current project.\r");
    let session = visible_session(&tui);
    wait_for_onboarding(&mut tui, session);
    let scan_before =
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("baseline scanner");
    let root_manifest_before = fs::read(checkout.join("Cargo.toml")).expect("baseline manifest");
    let unrelated_source_before =
        fs::read(checkout.join("crates/scriberig-query/src/lib.rs")).expect("unrelated source");
    let database = tui.fixture_path().join("scriberig.sqlite");
    let store = SqliteStore::open(&database).expect("open onboarding database");
    let project = store
        .list_projects()
        .expect("list projects")
        .into_iter()
        .find(|project| project.name == "scriberig")
        .expect("onboarded project");
    drop(store);
    let base = ProjectService::new(Arc::new(
        SqliteStore::open(&database).expect("project store"),
    ))
    .workspace(project.id)
    .expect("active semantic workspace");
    tui.send(b"Add support for ingesting PHP source files.\r");
    wait_for_branch_completion(
        &tui,
        session,
        "agent continuation",
        Duration::from_secs(900),
    );
    let continuation = tui.read_until("PHP_IMPLEMENTATION_RESULT");
    let saved_after_continuation = load_session(&database, session);
    assert!(
        continuation,
        "session after continuation: {saved_after_continuation:#?}; {}",
        tui.screen()
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
    assert_nested_branch_results(&saved, project.id, base.snapshot);
    let candidate = assert_candidate_result(&saved);
    let applied_snapshot = review_and_apply(
        &mut tui,
        &checkout,
        &database,
        project.id,
        base.snapshot,
        &candidate,
    );
    assert_applied_history_transition(&database, base.snapshot, applied_snapshot, &candidate);
    assert_unrelated_source_unchanged(&checkout, &unrelated_source_before);
    let fixture = tui.quit_retain_fixture();
    restart_and_verify(
        fixture,
        &checkout,
        &database,
        RestartEvidence {
            project: project.id,
            base_snapshot: base.snapshot,
            applied_snapshot,
            candidate: &candidate,
            session,
        },
    );
}

/// Identifies durable state checked after the candidate executable restarts.
///
/// Private members
/// - `project`: Onboarded project identity.
/// - `base_snapshot`: Exact candidate base snapshot.
/// - `applied_snapshot`: Applied successor snapshot.
/// - `candidate`: Durable candidate identity.
/// - `session`: Durable agent session identity.
///
/// Public members: None.
#[derive(Clone, Copy)]
struct RestartEvidence<'a> {
    /// Onboarded project identity.
    project: scriberig_core::ProjectId,
    /// Exact candidate base snapshot.
    base_snapshot: scriberig_core::SnapshotId,
    /// Applied successor snapshot.
    applied_snapshot: scriberig_core::SnapshotId,
    /// Durable candidate identity.
    candidate: &'a str,
    /// Durable agent session identity.
    session: scriberig_core::EntityId,
}

/// Reopens the candidate-built product and verifies durable applied project state.
///
/// - `fixture`: Retained temporary product fixture.
/// - `checkout`: Copied checkout containing the applied candidate.
/// - `database`: Durable product database path.
/// - `evidence`: Durable identities and snapshot lineage to verify.
fn restart_and_verify(
    fixture: std::path::PathBuf,
    checkout: &std::path::Path,
    database: &std::path::Path,
    evidence: RestartEvidence<'_>,
) {
    let candidate_target = tui_pty_support::workspace_target_directory().join("candidate-restart");
    let build = std::process::Command::new("cargo")
        .current_dir(checkout)
        .env("CARGO_TARGET_DIR", &candidate_target)
        .args(["build", "--bin", "srig", "--offline"])
        .output()
        .expect("build candidate checkout executable");
    assert!(
        build.status.success(),
        "candidate executable build failed: {}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let mut restarted = TuiPty::restart_candidate_checkout(
        fixture,
        checkout.to_path_buf(),
        candidate_target.join("debug/srig"),
    );
    assert!(
        restarted.read_until_for("snp_", Duration::from_secs(600)),
        "{}",
        restarted.diagnostic()
    );
    assert!(
        restarted.screen().contains(" @ snp_"),
        "{}",
        restarted.screen()
    );
    assert_eq!(
        assert_durable_candidate_effect(
            database,
            evidence.project,
            evidence.base_snapshot,
            evidence.candidate,
        ),
        evidence.applied_snapshot
    );
    let restored_session = load_session(database, evidence.session);
    assert_nested_branch_results(&restored_session, evidence.project, evidence.base_snapshot);
    assert_eq!(
        assert_candidate_result(&restored_session),
        evidence.candidate
    );
    assert!(
        SqliteStore::open(database)
            .expect("history store")
            .parents(
                ProjectService::new(Arc::new(
                    SqliteStore::open(database).expect("project store")
                ))
                .workspace(evidence.project)
                .expect("reopened workspace")
                .snapshot,
            )
            .expect("reimported history")
            .contains(&evidence.applied_snapshot)
    );
    assert_applied_php(checkout, database, evidence.project, evidence.base_snapshot);
    assert!(restarted.shutdown());
}

/// Reviews the durable candidate and explicitly applies it through the native TUI.
///
/// - `tui`: Live PTY session.
/// - `checkout`: Copied source checkout.
/// - `database`: Durable product database path.
/// - `project`: Onboarded project identity.
/// - `base_snapshot`: Exact candidate base.
/// - `candidate`: Durable candidate handle returned by the agent.
///
/// The authoritative snapshot captured immediately after application.
/// # Returns
/// The operation result.
fn review_and_apply(
    tui: &mut TuiPty,
    checkout: &std::path::Path,
    database: &std::path::Path,
    project: scriberig_core::ProjectId,
    base_snapshot: scriberig_core::SnapshotId,
    candidate: &str,
) -> scriberig_core::SnapshotId {
    candidate_review::open_candidate_review(tui);
    let review = tui.screen();
    assert!(review.contains("Validation: Passed"), "{review}");
    assert!(
        review.contains("profile: project-ingestion-e2e"),
        "{review}"
    );
    assert!(review.contains(&candidate[..20]), "{review}");
    let php_path = "crates/domains/scriberig-domain-php";
    for _ in 0..20 {
        if tui.screen().contains(php_path) {
            break;
        }
        tui.send(b"\x1b[B");
        let _ = tui.read_until_for(php_path, Duration::from_millis(150));
    }
    let create_review = tui.screen();
    assert!(create_review.contains(php_path), "{create_review}");
    assert!(create_review.contains("old:"), "{create_review}");
    assert!(create_review.contains("new:"), "{create_review}");
    let root_manifest = "update Cargo.toml";
    for _ in 0..20 {
        if tui.screen().contains(root_manifest) {
            break;
        }
        tui.send(b"\x1b[A");
        let _ = tui.read_until_for(root_manifest, Duration::from_millis(150));
    }
    let update_review = tui.screen();
    assert!(update_review.contains(root_manifest), "{update_review}");
    assert!(update_review.contains("old:"), "{update_review}");
    assert!(update_review.contains("new:"), "{update_review}");
    assert_candidate_conflicts_are_visible(
        tui,
        checkout,
        database,
        project,
        base_snapshot,
        candidate,
    );
    candidate_review::open_apply_confirmation(tui);
    candidate_review::confirm_candidate_apply(tui);
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        let file_exists = checkout
            .join("crates/domains/scriberig-domain-php/src/lib.rs")
            .is_file();
        let current = ProjectService::new(Arc::new(
            SqliteStore::open(database).expect("project store"),
        ))
        .workspace(project)
        .expect("applied project workspace");
        if file_exists && current.snapshot != base_snapshot {
            return assert_materialized_php_fixture(checkout, database, project, base_snapshot);
        }
        assert!(std::time::Instant::now() < deadline, "{}", tui.diagnostic());
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Confirms candidate construction left the canonical snapshot and checkout unchanged.
///
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
/// - `database`: Product database path.
/// - `session`: Exact session identity shown by the composer.
///
/// Persisted session with completed branch results.
/// # Returns
/// The operation result.
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
/// - `session`: Persisted agent session to inspect.
/// - `super`: Parent PTY test module.
///
/// Candidate identity exposed in the root result.
/// # Returns
/// The operation result.
fn assert_candidate_result(session: &scriberig_session::Session) -> String {
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
    root_result
        .result_handles
        .iter()
        .find_map(|handle| handle.strip_prefix("candidate:"))
        .expect("candidate result handle")
        .to_owned()
}
