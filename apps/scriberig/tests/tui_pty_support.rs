//! Provides a native pseudo-terminal driver for TUI integration tests.

use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::{
        Mutex, OnceLock,
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use vt100::Parser;

#[path = "tui_pty_support/lifecycle.rs"]
mod lifecycle;

#[path = "tui_pty_support/driver.rs"]
mod tui_pty_driver;
#[path = "tui_pty_support/fixture.rs"]
mod tui_pty_fixture;
#[path = "tui_pty_support/interaction.rs"]
mod tui_pty_interaction;
use tui_pty_fixture::{basic_fixture, copy_directory, read_output, temporary_fixture};

#[path = "tui_pty_support/self_hosting.rs"]
mod self_hosting;

/// Owns one real terminal session used by native TUI workflow tests.
pub struct TuiPty {
    fixture: PathBuf,
    project: PathBuf,
    master: Box<dyn portable_pty::MasterPty>,
    child: Box<dyn portable_pty::Child + Send>,
    writer: Box<dyn Write + Send>,
    receiver: Receiver<Vec<u8>>,
    parser: Parser,
    transcript: Vec<u8>,
}

impl TuiPty {
    /// Starts the native TUI in a fixed-size `ConPTY` session.
    ///
    /// # Returns
    /// A running TUI session with a temporary project fixture.
    ///
    /// # Panics
    /// Panics when the fixture, pseudo-terminal, or child process cannot be created.
    #[must_use]
    pub fn start() -> Self {
        Self::start_for_fixture(basic_fixture())
    }

    /// Starts the native TUI with an empty database and no selected project.
    ///
    /// # Returns
    /// A running TUI session with a fresh database and no project arguments.
    ///
    /// # Panics
    /// Panics when the pseudo-terminal or child process cannot be created.
    #[must_use]
    pub fn start_empty() -> Self {
        Self::start_with_project(temporary_fixture(), false)
    }

    /// Starts a fresh-database TUI in a Git-backed checkout with deterministic agent provider.
    ///
    /// # Returns
    /// A TUI process whose trusted bootstrap directory is a temporary Git project.
    ///
    /// # Panics
    /// Panics when the fixture, Git repository, PTY, or child process cannot be created.
    #[must_use]
    pub fn start_bootstrap() -> Self {
        let fixture = temporary_fixture();
        let checkout = tui_pty_fixture::bootstrap_project(&fixture);
        Self::start_configured(fixture, None, false, Some(checkout), true)
    }

    /// Reopens a bootstrap project by its registered checkout after a process restart.
    ///
    /// # Parameters
    /// - `fixture`: Temporary database and checkout owner directory.
    ///
    /// # Returns
    /// A new TUI process using the durable project and session state.
    #[must_use]
    pub fn restart_bootstrap(fixture: PathBuf) -> Self {
        let checkout = fixture.join("scriberig");
        Self::start_configured(fixture, Some(checkout.clone()), false, Some(checkout), true)
    }

    /// Starts the TUI against the checked-in Rust golden project copy.
    ///
    /// # Returns
    /// A running TUI session with the named Rust fixture.
    ///
    /// # Panics
    /// Panics when the fixture cannot be copied or the child cannot start.
    #[must_use]
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
        Self::start_configured(fixture, Some(project), true, None, false)
    }

    /// Restarts the native TUI against an existing golden fixture.
    ///
    /// # Parameters
    /// - `fixture`: Existing project and database directory.
    ///
    /// # Returns
    /// A running TUI session using the existing durable state.
    #[must_use]
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
    /// # Parameters
    /// - `fixture`: Prepared project directory.
    ///
    /// # Returns
    /// A running TUI session.
    ///
    /// # Panics
    /// Panics when the pseudo-terminal or child process cannot be created.
    fn start_for_fixture(fixture: PathBuf) -> Self {
        Self::start_with_project(fixture, true)
    }

    /// Starts the native process with optional project-import arguments.
    ///
    /// # Parameters
    /// - `fixture`: Directory used to hold the fresh or durable database.
    /// - `import_project`: Whether to pass the fixture as a project to import.
    ///
    /// # Returns
    /// A running TUI session.
    ///
    /// # Panics
    /// Panics when the pseudo-terminal or child process cannot be created.
    fn start_with_project(fixture: PathBuf, import_project: bool) -> Self {
        let project_path = import_project.then(|| fixture.clone());
        Self::start_configured(fixture, project_path, import_project, None, false)
    }

    /// Starts the native process with explicit runtime working directory and provider.
    ///
    /// # Parameters
    /// - `fixture`: Directory used to hold the fresh or durable database.
    /// - `project_path`: Optional project checkout passed to the product CLI.
    /// - `import_project`: Whether to pass the fixture as a project to import.
    /// - `working_directory`: Optional process working directory.
    /// - `deterministic_agent`: Whether to select the deterministic provider.
    ///
    /// # Returns
    /// A running native TUI session.
    ///
    /// # Panics
    /// Panics when the PTY or child process cannot be created.
    fn start_configured(
        fixture: PathBuf,
        project_path: Option<PathBuf>,
        import_project: bool,
        working_directory: Option<PathBuf>,
        deterministic_agent: bool,
    ) -> Self {
        Self::start_configured_scenario(
            fixture,
            project_path,
            import_project,
            working_directory,
            deterministic_agent,
            None,
            None,
        )
    }

    /// Starts the native process with an optional deterministic provider scenario.
    ///
    /// # Parameters
    /// - `fixture`: Directory used for the fresh database.
    /// - `project_path`: Optional project path passed to CLI.
    /// - `import_project`: Whether startup imports the project directly.
    /// - `working_directory`: Optional process working directory.
    /// - `deterministic_agent`: Whether a deterministic provider is selected.
    /// - `scenario`: Optional deterministic scenario identifier.
    /// - `executable`: Optional native executable override for restart gates.
    /// - `super`: Parent PTY support module visibility marker.
    ///
    /// # Returns
    /// A running native TUI process.
    ///
    /// # Panics
    /// Panics when the PTY or child process cannot be created.
    fn start_configured_scenario(
        fixture: PathBuf,
        project_path: Option<PathBuf>,
        import_project: bool,
        working_directory: Option<PathBuf>,
        deterministic_agent: bool,
        scenario: Option<&str>,
        executable: Option<PathBuf>,
    ) -> Self {
        let database = fixture.join("scriberig.sqlite");
        let product_project = project_path.clone().unwrap_or_else(|| fixture.clone());
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
        if let Some(project_path) = project_path {
            command.args(["--project", project_path.to_str().expect("project path")]);
            if import_project {
                command.arg("--import");
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
        }
    }

    /// Resizes the native terminal surface.
    ///
    /// # Parameters
    /// - `size`: New terminal dimensions.
    ///
    /// # Panics
    /// Panics when the PTY cannot be resized.
    pub fn resize(&mut self, size: PtySize) {
        self.master.resize(size).expect("resize pty");
        self.parser.set_size(size.rows, size.cols);
    }

    /// Returns the current virtual terminal screen as visible text.
    ///
    /// # Returns
    /// The current screen contents without terminal control sequences.
    #[must_use]
    pub fn screen(&self) -> String {
        self.parser.screen().contents()
    }

    /// Returns the fixture directory used for the database and checkout.
    ///
    /// # Returns
    /// The temporary fixture root.
    #[must_use]
    pub fn fixture_path(&self) -> &std::path::Path {
        &self.fixture
    }

    /// Reads the golden source file from the process-owned fixture.
    ///
    /// # Returns
    /// Current source bytes decoded as UTF-8.
    ///
    /// # Panics
    /// Panics when the fixture source cannot be read.
    #[must_use]
    pub fn source(&self) -> String {
        fs::read_to_string(self.project.join("src").join("lib.rs")).expect("read source")
    }

    /// Adds an external marker to the live Rust source.
    ///
    /// # Panics
    /// Panics when the fixture source cannot be read or written.
    pub fn external_edit(&self) {
        let path = self.project.join("src").join("lib.rs");
        let source = fs::read_to_string(&path).expect("read source");
        fs::write(path, format!("// external edit\n{source}")).expect("external edit");
    }

    /// Stops the process while retaining its fixture for a restart test.
    ///
    /// # Parameters
    /// - `self`: Running PTY session.
    ///
    /// # Returns
    /// The retained fixture directory.
    ///
    /// # Panics
    /// Panics when the child cannot be polled or stopped.
    #[must_use]
    pub fn quit_retain_fixture(mut self) -> PathBuf {
        self.send(b"q");
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if self.child.try_wait().expect("poll tui").is_some() {
                break;
            }
            if Instant::now() >= deadline {
                self.child.kill().expect("kill tui");
                self.child.wait().expect("wait for tui");
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        let fixture = self.fixture.clone();
        std::mem::forget(self);
        fixture
    }

    /// Describes the last virtual screen and raw terminal output.
    ///
    /// # Returns
    /// Diagnostic screen and transcript text for a failed assertion.
    #[must_use]
    pub fn diagnostic(&self) -> String {
        format!(
            "screen: {}; transcript: {}",
            self.screen(),
            String::from_utf8_lossy(&self.transcript)
        )
    }
}
/// Serializes native PTY sessions for Windows `ConPTY` stability.
///
/// # Returns
/// The process-wide PTY test lock guard.
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
