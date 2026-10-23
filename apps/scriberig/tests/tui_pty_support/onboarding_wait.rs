//! Waits for durable completion of the shared project onboarding flow.

use std::path::Path;
use std::time::{Duration, Instant};

use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;

use super::TuiPty;

/// Waits for the onboarding branch to durably complete project import.
///
/// - `tui`: Active native TUI process owning the session database.
/// - `session`: Exact onboarding session identity.
pub(super) fn wait_for_onboarding(tui: &mut TuiPty, session: scriberig_core::EntityId) {
    let database = tui.fixture_path().join("scriberig.sqlite");
    let deadline = Instant::now() + Duration::from_secs(120);
    let store = open_session_store_until(
        &database,
        deadline,
        "onboarding session store stayed locked",
    );
    loop {
        let saved = match store.load(session) {
            Ok(Some(saved)) => saved,
            Ok(None) => {
                assert!(
                    Instant::now() < deadline,
                    "onboarding session was not saved"
                );
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(error) if error.contains("database is locked") => {
                assert!(
                    Instant::now() < deadline,
                    "onboarding database stayed locked"
                );
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(error) => panic!("read onboarding session: {error}"),
        };
        let root_completed = saved.branches.iter().any(|branch| {
            branch.title == "agent root"
                && branch.status == scriberig_session::BranchStatus::Completed
        });
        let import_completed = saved.branches.iter().any(|branch| {
            branch.status == scriberig_session::BranchStatus::Completed
                && branch
                    .tool_invocations
                    .iter()
                    .any(|call| call.name == "project_import")
        });
        if root_completed && import_completed {
            assert!(
                tui.read_until_next("Enter send"),
                "TUI did not return to its conversation composer: {}",
                tui.diagnostic()
            );
            return;
        }
        let branches: Vec<String> = saved
            .branches
            .iter()
            .map(|branch| format!("{}:{:?}:{:?}", branch.title, branch.status, branch.outcome))
            .collect();
        assert!(
            Instant::now() < deadline,
            "onboarding branch did not complete: {branches:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Waits until one named durable branch reaches completion.
///
/// - `tui`: Active TUI process owning the session database.
/// - `session`: Exact durable session identity.
/// - `title`: Stable branch title to wait for.
/// - `timeout`: Maximum time to wait for completion.
#[allow(dead_code)]
pub(super) fn wait_for_branch_completion(
    tui: &TuiPty,
    session: scriberig_core::EntityId,
    title: &str,
    timeout: Duration,
) {
    let database = tui.fixture_path().join("scriberig.sqlite");
    let deadline = Instant::now() + timeout;
    let store = open_session_store_until(&database, deadline, "branch session store stayed locked");
    loop {
        let saved = match store.load(session) {
            Ok(Some(saved)) => saved,
            Ok(None) => {
                assert!(Instant::now() < deadline, "branch session was not saved");
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(error) if error.contains("database is locked") => {
                assert!(
                    Instant::now() < deadline,
                    "branch session store stayed locked"
                );
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(error) => panic!("read branch session: {error}"),
        };
        if let Some(branch) = saved.branches.iter().find(|branch| branch.title == title) {
            if branch.status == scriberig_session::BranchStatus::Completed {
                return;
            }
            assert!(
                branch.status == scriberig_session::BranchStatus::Active,
                "branch {title:?} terminated as {:?}: {:?}; tools: {:?}",
                branch.status,
                branch.outcome,
                branch
                    .tool_invocations
                    .iter()
                    .map(|invocation| invocation.name.as_str())
                    .collect::<Vec<_>>()
            );
        }
        let branches = saved
            .branches
            .iter()
            .map(|branch| format!("{}:{:?}", branch.title, branch.status))
            .collect::<Vec<_>>();
        assert!(
            Instant::now() < deadline,
            "branch {title} did not complete; durable branches: {branches:?}; screen: {}",
            tui.screen()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Opens a SQLite session store while retrying transient writer contention.
///
/// - `database`: Session database path.
/// - `deadline`: Maximum time allowed for a writer to release the database.
/// - `timeout_message`: Failure message used when the deadline expires.
///
/// # Returns
/// The opened session store.
///
/// # Panics
/// Panics when a non-locking database error occurs or the database stays locked.
fn open_session_store_until(
    database: &Path,
    deadline: Instant,
    timeout_message: &str,
) -> SqliteSessionStore {
    loop {
        match SqliteSessionStore::open(database) {
            Ok(store) => return store,
            Err(error) if error.to_string().contains("database is locked") => {
                assert!(Instant::now() < deadline, "{timeout_message}");
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => panic!("open session store: {error}"),
        }
    }
}

/// Reads the durable session identity from the local session store.
///
/// - `tui`: Active TUI process owning the local session store.
///
/// Session identity shown in the composer title.
#[allow(dead_code)]
/// # Returns
/// The operation result.
pub(super) fn visible_session(tui: &TuiPty) -> scriberig_core::EntityId {
    tui.stored_session_id()
}
