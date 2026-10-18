//! Waits for durable completion of the shared project onboarding flow.

use std::time::{Duration, Instant};

use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;

use super::TuiPty;

/// Waits for the onboarding branch to durably complete project import.
///
/// # Parameters
/// - `tui`: Active native TUI process owning the session database.
/// - `session`: Exact onboarding session identity.
pub(super) fn wait_for_onboarding(tui: &TuiPty, session: scriberig_core::EntityId) {
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
        if saved.branches.iter().any(|branch| {
            branch.status == scriberig_session::BranchStatus::Completed
                && branch
                    .tool_invocations
                    .iter()
                    .any(|call| call.name == "project_import")
        }) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "onboarding branch did not complete"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Parses the durable session identity displayed by the agent composer.
///
/// # Parameters
/// - `screen`: Current rendered terminal screen.
///
/// # Returns
/// Session identity shown in the composer title.
#[allow(dead_code)]
pub(super) fn visible_session(screen: &str) -> scriberig_core::EntityId {
    screen
        .split("Agent composer · ")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .expect("session visible in composer")
        .parse()
        .expect("session identity")
}
