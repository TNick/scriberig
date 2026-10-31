//! Verifies cancellation terminates isolated candidate validation in the native product.

#[allow(dead_code)]
mod tui_pty_support;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_session::{BranchStatus, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;
use tui_pty_support::onboarding_wait::wait_for_onboarding;
use tui_pty_support::{TuiPty, test_lock};

/// Cancels the real TUI while Cargo validates an isolated project candidate.
#[test]
fn tui_cancels_php_validation_without_project_or_live_source_mutation() {
    let _lock = test_lock();
    let mut tui = TuiPty::start_self_hosting();
    assert!(tui.read_until("Enter send"), "{}", tui.diagnostic());
    let checkout = tui.fixture_path().join("scriberig");
    let manifest = fs::read(checkout.join("Cargo.toml")).expect("baseline manifest");
    let scanner =
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("baseline scanner");

    tui.send(b"Onboard the current project.\r");
    let session = visible_session(&tui);
    wait_for_onboarding(&mut tui, session);

    let database = tui.fixture_path().join("scriberig.sqlite");
    let project = SqliteStore::open(&database)
        .expect("project store")
        .list_projects()
        .expect("project list")
        .into_iter()
        .find(|record| record.name == "scriberig")
        .expect("onboarded project")
        .id;
    let base = ProjectService::new(std::sync::Arc::new(
        SqliteStore::open(&database).expect("reopen project store"),
    ))
    .workspace(project)
    .expect("active project workspace")
    .snapshot;
    let preexisting = validation_workspaces();
    tui.send(b"Add support for ingesting PHP source files.\r");

    let validation_root = wait_for_candidate_validation(&preexisting);
    assert!(validation_root.join("Cargo.toml").is_file());
    assert!(
        validation_root
            .join("crates/domains/scriberig-domain-php/Cargo.toml")
            .is_file()
    );
    cancel_active_agent(&mut tui);

    let saved = wait_for_cancellation(&database, session);
    assert!(saved.branches.iter().any(|branch| {
        branch.status == BranchStatus::Abandoned
            && branch
                .result
                .as_ref()
                .is_some_and(|result| result.result_kind.as_deref() == Some("agent-cancelled"))
    }));
    assert_eq!(
        ProjectService::new(std::sync::Arc::new(
            SqliteStore::open(&database).expect("reopen project store"),
        ))
        .workspace(project)
        .expect("project remains active")
        .snapshot,
        base
    );
    assert_eq!(
        fs::read(checkout.join("Cargo.toml")).expect("manifest"),
        manifest
    );
    assert_eq!(
        fs::read(checkout.join("crates/scriberig-project/src/scan.rs")).expect("scanner"),
        scanner
    );
    assert!(
        !checkout
            .join("crates/domains/scriberig-domain-php")
            .exists()
    );
    assert!(
        !validation_root.exists(),
        "cancelled candidate workspace remains on disk"
    );
    let fixture = tui.quit_retain_fixture();
    fs::remove_dir_all(fixture).expect("remove validation cancellation fixture");
}

/// Captures candidate validation workspaces already present before the scenario.
///
/// Exact temporary workspace paths to exclude from this run's observation.
/// # Returns
/// The operation result.
fn validation_workspaces() -> HashSet<PathBuf> {
    fs::read_dir(std::env::temp_dir())
        .expect("read temporary directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("scriberig-project-validation-"))
        })
        .collect()
}

/// Waits until the candidate's isolated PHP sources have reached Cargo validation.
///
/// - `preexisting`: Workspaces excluded from the active scenario.
///
/// The newly materialized candidate validation workspace.
/// # Returns
/// The operation result.
fn wait_for_candidate_validation(preexisting: &HashSet<PathBuf>) -> PathBuf {
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        if let Ok(entries) = fs::read_dir(std::env::temp_dir()) {
            for entry in entries.filter_map(Result::ok) {
                let root = entry.path();
                if preexisting.contains(&root) || !is_candidate_validation_workspace(&root) {
                    continue;
                }
                let identifier = root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.strip_prefix("scriberig-project-validation-"));
                if identifier.is_some_and(|id| {
                    candidate_target_root()
                        .join(format!("scriberig-candidate-target-{id}"))
                        .exists()
                }) {
                    return root;
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "isolated candidate Cargo validation did not start"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

/// Resolves the configured root for isolated candidate Cargo artifacts.
///
/// The application places candidate targets beside the product target when
/// `CARGO_TARGET_DIR` is configured, and otherwise uses the system temp dir.
///
/// # Returns
/// The candidate artifact root.
fn candidate_target_root() -> PathBuf {
    tui_pty_support::workspace_target_directory()
}

/// Matches the validation observer's artifact root to the spawned TUI target.
#[test]
fn candidate_target_observer_uses_the_spawned_tui_target_directory() {
    assert_eq!(
        candidate_target_root(),
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(
            || {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join("target")
            },
            PathBuf::from,
        )
    );
}

/// Checks whether a temporary directory contains the expected PHP candidate.
///
/// - `root`: Candidate workspace path to inspect.
///
/// `true` when the candidate workspace contains Cargo and PHP domain manifests.
/// # Returns
/// The operation result.
fn is_candidate_validation_workspace(root: &Path) -> bool {
    root.join("Cargo.toml").is_file()
        && root
            .join("crates/domains/scriberig-domain-php/Cargo.toml")
            .is_file()
}

/// Cancels the active agent from the native TUI operations view.
///
/// - `tui`: Active product process.
fn cancel_active_agent(tui: &mut TuiPty) {
    tui.send_slow(b"/operations\r");
    assert!(tui.read_until("Operation:"), "{}", tui.diagnostic());
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

/// Waits until the session store persists a terminal cancellation result.
///
/// - `database`: Durable product database.
/// - `session`: Agent session identity.
///
/// The durable session after the cancelled operation completes.
/// # Returns
/// The operation result.
fn wait_for_cancellation(database: &Path, session: EntityId) -> scriberig_session::Session {
    let store = SqliteSessionStore::open(database).expect("session store");
    let deadline = Instant::now() + Duration::from_secs(30);
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

/// Parses the durable session identity displayed in the TUI composer.
///
/// - `tui`: Active native TUI process.
///
/// Stable session entity identity.
/// # Returns
/// The operation result.
fn visible_session(tui: &TuiPty) -> EntityId {
    tui.stored_session_id()
}
