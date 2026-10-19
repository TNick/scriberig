//! Waits for durable completion of the shared project onboarding flow.

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
    let store = SqliteSessionStore::open(&database).expect("open onboarding session store");
    let deadline = Instant::now() + Duration::from_secs(120);
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
        assert!(
            Instant::now() < deadline,
            "onboarding branch did not complete"
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
    let store = SqliteSessionStore::open(&database).expect("open branch session store");
    let deadline = Instant::now() + timeout;
    loop {
        let saved = store
            .load(session)
            .expect("read branch session")
            .expect("branch session exists");
        if let Some(branch) = saved.branches.iter().find(|branch| branch.title == title) {
            if branch.status == scriberig_session::BranchStatus::Completed {
                return;
            }
            assert!(
                branch.status == scriberig_session::BranchStatus::Active,
                "branch {title:?} terminated as {:?}: {:?}",
                branch.status,
                branch.outcome
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
