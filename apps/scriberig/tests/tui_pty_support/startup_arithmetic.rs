//! Configures deterministic arithmetic TUI test processes.

use super::{TuiPty, startup_options::ProjectStartupOptions, tui_pty_fixture::temporary_fixture};

impl TuiPty {
    /// Starts a clean TUI using a mock model that answers arithmetic with `2`.
    ///
    /// # Returns
    /// The running pseudo-terminal session.
    #[must_use]
    pub fn start_with_arithmetic_model() -> Self {
        let fixture = temporary_fixture();
        Self::start_configured_scenario(
            fixture,
            ProjectStartupOptions {
                path: None,
                import: false,
                git_tracked: false,
            },
            None,
            true,
            Some("arithmetic"),
            None,
        )
    }

    /// Starts the arithmetic TUI with one abandoned conversation already saved.
    ///
    /// # Returns
    /// The running pseudo-terminal session.
    ///
    /// # Panics
    /// Panics when the fixture cannot be created or persisted.
    #[must_use]
    pub fn start_with_arithmetic_model_and_abandoned_history() -> Self {
        let fixture = temporary_fixture();
        let session = scriberig_session::Session {
            id: scriberig_core::EntityId::new(),
            branches: vec![{
                let mut branch = scriberig_session::Branch::new(
                    scriberig_core::EntityId::new(),
                    "Old abandoned conversation",
                )
                .expect("create abandoned conversation branch");
                branch.status = scriberig_session::BranchStatus::Abandoned;
                branch
            }],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        };
        let store =
            scriberig_session_sqlite::SqliteSessionStore::open(fixture.join("scriberig.sqlite"))
                .expect("open abandoned session fixture");
        scriberig_session::SessionStore::save(&store, &session)
            .expect("save abandoned conversation fixture");
        drop(store);
        Self::start_configured_scenario(
            fixture,
            ProjectStartupOptions {
                path: None,
                import: false,
                git_tracked: false,
            },
            None,
            true,
            Some("arithmetic"),
            None,
        )
    }
}
