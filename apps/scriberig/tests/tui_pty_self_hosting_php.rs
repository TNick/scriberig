//! Verifies PHP self-extension proposal through a copied real checkout and PTY.

#[path = "tui_pty_support/self_hosting_php_assertions.rs"]
mod assertions;
#[allow(dead_code)]
mod tui_pty_support;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use assertions::{
    assert_applied_php, assert_candidate_conflicts_are_visible, assert_durable_candidate_effect,
    assert_materialized_php_fixture, assert_tui_history_transition,
};
use scriberig_project::ProjectService;
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store::SnapshotStore;
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
    let continuation =
        tui.read_until_for("agent continuation [Completed]", Duration::from_secs(300));
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
    assert!(saved.branches.iter().any(|branch| {
        branch
            .purpose
            .as_deref()
            .is_some_and(|purpose| purpose.contains("determine minimal extension points"))
    }));
    let candidate = assert_candidate_result(&saved);
    assert!(saved.branches.iter().any(|branch| {
        branch
            .purpose
            .as_deref()
            .is_some_and(|purpose| purpose.contains("implement PHP ingestion"))
    }));
    let applied_snapshot = review_and_apply(
        &mut tui,
        &checkout,
        &database,
        project.id,
        base.snapshot,
        &candidate,
    );
    assert_tui_history_transition(&mut tui, base.snapshot, applied_snapshot);
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
/// # Parameters
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
    let build = std::process::Command::new("cargo")
        .current_dir(checkout)
        .env("CARGO_TARGET_DIR", checkout.join("target"))
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
        checkout.join("target/debug/srig"),
    );
    assert!(
        restarted.read_until_for("snp_", Duration::from_secs(120)),
        "{}",
        restarted.diagnostic()
    );
    assert!(
        restarted.screen().contains("project:"),
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
/// # Parameters
/// - `tui`: Live PTY session.
/// - `checkout`: Copied source checkout.
/// - `database`: Durable product database path.
/// - `project`: Onboarded project identity.
/// - `base_snapshot`: Exact candidate base.
/// - `candidate`: Durable candidate handle returned by the agent.
///
/// # Returns
/// The authoritative snapshot captured immediately after application.
fn review_and_apply(
    tui: &mut TuiPty,
    checkout: &std::path::Path,
    database: &std::path::Path,
    project: scriberig_core::ProjectId,
    base_snapshot: scriberig_core::SnapshotId,
    candidate: &str,
) -> scriberig_core::SnapshotId {
    tui.send(b":");
    assert!(tui.read_until("Search:"), "{}", tui.diagnostic());
    tui.send(b"review-candidate\r");
    assert!(tui.read_until("Change review"), "{}", tui.diagnostic());
    let review = tui.screen();
    assert!(review.contains("validation: Passed"), "{review}");
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
    tui.send(b"\r");
    assert!(tui.read_until("Enter confirm"), "{}", tui.diagnostic());
    tui.send(b"\r");
    assert!(
        tui.read_until_for("materialized", Duration::from_secs(120)),
        "{}",
        tui.diagnostic()
    );
    let successor = assert_materialized_php_fixture(checkout, database, project, base_snapshot);
    let screen = tui.screen();
    assert!(
        screen.contains(&successor.to_string()[..20]),
        "active TUI did not refresh to {successor}: {screen}"
    );
    successor
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
/// - `super`: Parent PTY test module.
///
/// # Returns
/// Candidate identity exposed in the root result.
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
