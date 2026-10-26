//! Implements TUI process startup and fixture configuration.

use std::{io::Write, path::PathBuf, sync::mpsc, thread};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use vt100::Parser;

use super::tui_pty_fixture::{basic_fixture, copy_directory, read_output, temporary_fixture};
use super::{
    TuiPty, startup_options::ProjectStartupOptions, tui_pty_fixture, workspace_target_directory,
};

impl TuiPty {
    /// Starts the native TUI in a fixed-size `ConPTY` session.
    ///
    /// A running TUI session with a temporary project fixture.
    ///
    /// # Panics
    /// Panics when the fixture, pseudo-terminal, or child process cannot be created.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn start() -> Self {
        Self::start_for_fixture(basic_fixture())
    }

    /// Starts the TUI with a deterministic provider and supplied fake models.
    ///
    /// - `model_catalog`: Trusted test-only catalog containing fake profiles.
    ///
    /// A running TUI with an imported project and deterministic agent provider.
    ///
    /// # Panics
    /// Panics when the fixture or child process cannot be created.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn start_with_model_catalog(model_catalog: &str) -> Self {
        let fixture = temporary_fixture();
        let checkout = tui_pty_fixture::bootstrap_project(&fixture);
        Self::start_configured_scenario_with_catalog(
            fixture,
            ProjectStartupOptions {
                path: None,
                import: false,
                git_tracked: false,
            },
            Some(checkout),
            true,
            Some("bootstrap"),
            None,
            Some(model_catalog),
        )
    }

    /// Starts a fresh-database TUI in a Git-backed checkout with deterministic agent provider.
    ///
    /// A TUI process whose trusted bootstrap directory is a temporary Git project.
    ///
    /// # Panics
    /// Panics when the fixture, Git repository, PTY, or child process cannot be created.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn start_bootstrap() -> Self {
        let fixture = temporary_fixture();
        let checkout = tui_pty_fixture::bootstrap_project(&fixture);
        Self::start_configured(fixture, None, false, Some(checkout), true)
    }

    /// Reopens a bootstrap project by its registered checkout after a process restart.
    ///
    /// - `fixture`: Temporary database and checkout owner directory.
    ///
    /// A new TUI process using the durable project and session state.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn restart_bootstrap(fixture: PathBuf) -> Self {
        let checkout = fixture.join("scriberig");
        Self::start_configured(fixture, Some(checkout.clone()), false, Some(checkout), true)
    }

    /// Starts the TUI against the checked-in Rust golden project copy.
    ///
    /// A running TUI session with the named Rust fixture.
    ///
    /// # Panics
    /// Panics when the fixture cannot be copied or the child cannot start.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn start_golden() -> Self {
        let fixture = temporary_fixture();
        let project = fixture.join("project");
        copy_directory(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("tests")
                .join("fixtures")
                .join("golden-rust-project"),
            &project,
        );
        Self::start_configured(fixture, Some(project), true, None, true)
    }

    /// Restarts the native TUI against an existing golden fixture.
    ///
    /// - `fixture`: Existing project and database directory.
    ///
    /// A running TUI session using the existing durable state.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn restart(fixture: PathBuf) -> Self {
        let project = fixture.join("project");
        if project.is_dir() {
            Self::start_configured(fixture, Some(project), true, None, false)
        } else {
            Self::start_for_fixture(fixture)
        }
    }

    /// Starts the native process for one prepared fixture.
    ///
    /// - `fixture`: Prepared project directory.
    ///
    /// A running TUI session.
    ///
    /// # Panics
    /// Panics when the pseudo-terminal or child process cannot be created.
    /// # Returns
    /// The operation result.
    fn start_for_fixture(fixture: PathBuf) -> Self {
        Self::start_with_project(fixture, true)
    }

    /// Starts the native process with optional project-import arguments.
    ///
    /// - `fixture`: Directory used to hold the fresh or durable database.
    /// - `import_project`: Whether to pass the fixture as a project to import.
    ///
    /// A running TUI session.
    ///
    /// # Panics
    /// Panics when the pseudo-terminal or child process cannot be created.
    /// # Returns
    /// The operation result.
    fn start_with_project(fixture: PathBuf, import_project: bool) -> Self {
        let project_path = import_project.then(|| fixture.clone());
        Self::start_configured(fixture, project_path, import_project, None, false)
    }

    /// Starts the native process with explicit runtime working directory and provider.
    ///
    /// - `fixture`: Directory used to hold the fresh or durable database.
    /// - `project_path`: Optional project checkout passed to the product CLI.
    /// - `import_project`: Whether to pass the fixture as a project to import.
    /// - `working_directory`: Optional process working directory.
    /// - `deterministic_agent`: Whether to select the deterministic provider.
    ///
    /// A running native TUI session.
    ///
    /// # Panics
    /// Panics when the PTY or child process cannot be created.
    /// # Returns
    /// The running TUI process.
    /// # Returns
    /// The operation result.
    fn start_configured(
        fixture: PathBuf,
        project_path: Option<PathBuf>,
        import_project: bool,
        working_directory: Option<PathBuf>,
        deterministic_agent: bool,
    ) -> Self {
        Self::start_configured_scenario(
            fixture,
            ProjectStartupOptions {
                path: project_path,
                import: import_project,
                git_tracked: false,
            },
            working_directory,
            deterministic_agent,
            None,
            None,
        )
    }

    /// Starts the native process with an optional deterministic provider scenario.
    ///
    /// - `fixture`: Directory used for the fresh database.
    /// - `project`: Optional project behavior selected for startup.
    /// - `working_directory`: Optional process working directory.
    /// - `deterministic_agent`: Whether a deterministic provider is selected.
    /// - `scenario`: Optional deterministic scenario identifier.
    /// - `executable`: Optional native executable override for restart gates.
    /// - `super`: Parent PTY support module visibility marker.
    ///
    /// A running native TUI process.
    ///
    /// # Panics
    /// Panics when the PTY or child process cannot be created.
    /// # Returns
    /// The operation result.
    pub(super) fn start_configured_scenario(
        fixture: PathBuf,
        project: ProjectStartupOptions,
        working_directory: Option<PathBuf>,
        deterministic_agent: bool,
        scenario: Option<&str>,
        executable: Option<PathBuf>,
    ) -> Self {
        Self::start_configured_scenario_with_catalog(
            fixture,
            project,
            working_directory,
            deterministic_agent,
            scenario,
            executable,
            None,
        )
    }

    /// Starts the native process with an optional trusted test model catalog.
    ///
    /// - `fixture`: Directory used for the fresh database.
    /// - `project`: Optional project behavior selected for startup.
    /// - `working_directory`: Optional process working directory.
    /// - `deterministic_agent`: Whether to select the deterministic provider.
    /// - `scenario`: Optional deterministic scenario identifier.
    /// - `executable`: Optional native executable override for restart gates.
    /// - `model_catalog`: Optional fake model catalog passed to the child.
    ///
    /// A running native TUI process.
    ///
    /// # Panics
    /// Panics when the PTY or child process cannot be created.
    /// # Returns
    /// The running TUI process.
    fn start_configured_scenario_with_catalog(
        fixture: PathBuf,
        project: ProjectStartupOptions,
        working_directory: Option<PathBuf>,
        deterministic_agent: bool,
        scenario: Option<&str>,
        executable: Option<PathBuf>,
        model_catalog: Option<&str>,
    ) -> Self {
        Self::start_configured_scenario_with_catalog_and_environment(
            fixture,
            project,
            working_directory,
            deterministic_agent,
            scenario,
            executable,
            model_catalog,
            &[],
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "PTY setup keeps independent process-boundary inputs explicit"
    )]
    /// Starts the native process with explicit extra child environment values.
    ///
    /// - `fixture`: Directory used for the fresh database.
    /// - `project`: Optional project behavior selected for startup.
    /// - `working_directory`: Optional process working directory.
    /// - `deterministic_agent`: Whether to select the deterministic provider.
    /// - `scenario`: Optional deterministic scenario identifier.
    /// - `executable`: Optional native executable override for restart gates.
    /// - `model_catalog`: Optional fake model catalog passed to the child.
    /// - `environment`: Additional child environment entries.
    ///
    /// # Returns
    /// The configured product process and captured terminal output.
    ///
    /// # Panics
    /// Panics when the PTY or child process cannot be created.
    pub(super) fn start_configured_scenario_with_catalog_and_environment(
        fixture: PathBuf,
        project: ProjectStartupOptions,
        working_directory: Option<PathBuf>,
        deterministic_agent: bool,
        scenario: Option<&str>,
        executable: Option<PathBuf>,
        model_catalog: Option<&str>,
        environment: &[(&str, String)],
    ) -> Self {
        let database = fixture.join("scriberig.sqlite");
        let product_project = project.path.clone().unwrap_or_else(|| fixture.clone());
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 40,
                cols: 140,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("open pty");
        let mut command = CommandBuilder::new(
            executable.unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_srig"))),
        );
        command.env("CARGO_TARGET_DIR", workspace_target_directory());
        if let Some(catalog) = model_catalog {
            command.env("SCRIBE_AGENT_MODEL_CATALOG", catalog);
        }
        for (name, value) in environment {
            command.env(name, value);
        }
        for name in ["SCRIBE_AGENT_PACKAGE_ROOTS", "SCRIBE_AGENT_PLUGIN_FILES"] {
            command.env_remove(name);
        }
        if let Some(temp) = std::env::var_os("TEMP") {
            command.env("TEMP", temp);
        }
        if let Some(tmp) = std::env::var_os("TMP") {
            command.env("TMP", tmp);
        }
        if let Some(path) = working_directory {
            command.cwd(path);
        }
        if deterministic_agent {
            command.env("SCRIBE_AGENT_PROVIDER", "deterministic");
            command.env("SCRIBE_AGENT_SCENARIO", scenario.unwrap_or("bootstrap"));
        }
        if matches!(
            scenario,
            Some(
                "self_hosting_php_cancellation"
                    | "self_hosting_php_discovery_cancellation"
                    | "self_hosting_php_implementation_cancellation"
            )
        ) {
            command.env(
                "SCRIBE_AGENT_TEST_PROVIDER_GATE",
                fixture
                    .join("agent-provider.release")
                    .to_str()
                    .expect("provider release marker path"),
            );
        }
        command.args(["tui", "--db", database.to_str().expect("database path")]);
        if let Some(project_path) = project.path {
            command.args(["--project", project_path.to_str().expect("project path")]);
            if project.import {
                command.arg("--import");
            }
            if project.git_tracked {
                command.arg("--git-tracked");
            }
        }
        let child = pair.slave.spawn_command(command).expect("spawn tui");
        let master = pair.master;
        let mut writer = master.take_writer().expect("take writer");
        let reader = master.try_clone_reader().expect("clone reader");
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || read_output(reader, sender));
        writer.flush().expect("flush terminal writer");
        Self {
            project: product_project,
            fixture,
            master,
            child,
            writer,
            receiver,
            parser: Parser::new(40, 140, 0),
            transcript: Vec::new(),
            query_cursor: 0,
        }
    }

    /// Starts the native TUI with an empty database and no selected project.
    ///
    /// A running TUI session with a fresh database and no project arguments.
    ///
    /// # Panics
    /// Panics when the pseudo-terminal or child process cannot be created.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn start_empty() -> Self {
        Self::start_with_project(super::tui_pty_fixture::temporary_fixture(), false)
    }
}
