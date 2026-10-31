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
#[allow(dead_code, reason = "shared by separate self-hosting PTY targets")]
pub(crate) fn wait_for_onboarding(tui: &mut TuiPty, session: scriberig_core::EntityId) {
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
        if let Some(failure) = onboarding_root_failure(&saved) {
            panic!("{failure}; {}", tui.diagnostic());
        }
        let branches: Vec<String> = saved
            .branches
            .iter()
            .map(|branch| format!("{}:{:?}:{:?}", branch.title, branch.status, branch.outcome))
            .collect();
        let debug_log = std::fs::read_to_string(tui.fixture_path().join("tui-debug.log"))
            .unwrap_or_else(|error| format!("debug log unavailable: {error}"));
        let agent_log = std::fs::read_to_string(tui.fixture_path().join("agent-trace.log"))
            .unwrap_or_else(|error| format!("agent trace unavailable: {error}"));
        assert!(
            Instant::now() < deadline,
            "onboarding branch did not complete: {branches:?}; TUI debug log: {debug_log}; agent trace: {agent_log}; {}",
            tui.diagnostic()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Summarizes a terminal root branch before the onboarding wait deadline.
///
/// - `saved`: Latest durable onboarding session.
///
/// A failure diagnostic when the root exists and is no longer active.
///
/// # Returns
/// The failure diagnostic, or `None` while onboarding can continue.
pub(crate) fn onboarding_root_failure(saved: &scriberig_session::Session) -> Option<String> {
    let root = saved
        .branches
        .iter()
        .find(|branch| branch.title == "agent root")
        .filter(|branch| branch.status != scriberig_session::BranchStatus::Active)?;
    let truncate = |value: &str| {
        let mut chars = value.chars();
        let excerpt = chars.by_ref().take(160).collect::<String>();
        if chars.next().is_some() {
            format!("{excerpt}…")
        } else {
            excerpt
        }
    };
    let branches = saved
        .branches
        .iter()
        .map(|branch| {
            let tools = branch
                .tool_invocations
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>();
            let requests = branch
                .model_invocations
                .iter()
                .rev()
                .take(2)
                .map(|invocation| {
                    let messages = invocation.request.as_ref().map(|request| {
                        request
                            .messages
                            .iter()
                            .rev()
                            .take(2)
                            .map(|message| (message.role.as_str(), truncate(&message.content)))
                            .collect::<Vec<_>>()
                    });
                    (
                        invocation.provider_id.as_deref(),
                        invocation.model.as_deref(),
                        messages,
                    )
                })
                .collect::<Vec<_>>();
            format!(
                "{}:{:?}:{:?}:tools={tools:?}:requests={requests:?}",
                branch.title, branch.status, branch.outcome
            )
        })
        .collect::<Vec<_>>();
    Some(format!(
        "onboarding root terminated as {:?}: {:?}; branches: {branches:?}",
        root.status, root.outcome
    ))
}

/// Waits until one named durable branch reaches completion.
///
/// - `tui`: Active TUI process owning the session database.
/// - `session`: Exact durable session identity.
/// - `title`: Stable branch title to wait for.
/// - `timeout`: Maximum time to wait for completion.
#[allow(dead_code)]
pub(crate) fn wait_for_branch_completion(
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
pub(crate) fn visible_session(tui: &TuiPty) -> scriberig_core::EntityId {
    tui.stored_session_id()
}

#[cfg(test)]
mod tests {
    use super::onboarding_root_failure;
    use scriberig_core::EntityId;
    use scriberig_session::{Branch, BranchStatus, Session, SessionExecutionSelection};

    /// Confirms an abandoned durable root is surfaced before the wait deadline.
    #[test]
    fn abandoned_root_has_a_terminal_diagnostic() {
        let mut root = Branch::new(EntityId::new(), "agent root").expect("root branch");
        root.status = BranchStatus::Abandoned;
        root.outcome = Some("agent-failure".to_owned());
        let session = Session {
            id: EntityId::new(),
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection::default(),
        };

        let diagnostic = onboarding_root_failure(&session).expect("terminal diagnostic");

        assert!(diagnostic.contains("terminated as Abandoned"));
        assert!(diagnostic.contains("agent-failure"));
    }

    /// Confirms an active durable root remains eligible for further progress.
    #[test]
    fn active_root_has_no_terminal_diagnostic() {
        let root = Branch::new(EntityId::new(), "agent root").expect("root branch");
        let session = Session {
            id: EntityId::new(),
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection::default(),
        };

        assert!(onboarding_root_failure(&session).is_none());
    }
}
