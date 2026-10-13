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

#[path = "tui_pty_support/fixture.rs"]
mod tui_pty_fixture;
#[path = "tui_pty_support/interaction.rs"]
mod tui_pty_interaction;
use tui_pty_fixture::{basic_fixture, copy_directory, read_output, temporary_fixture};

/// Owns one real terminal session used by native TUI workflow tests.
pub struct TuiPty {
    fixture: PathBuf,
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
        copy_directory(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("tests")
                .join("fixtures")
                .join("golden-rust-project"),
            &fixture,
        );
        Self::start_for_fixture(fixture)
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
        Self::start_for_fixture(fixture)
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
        let database = fixture.join("scriberig.sqlite");
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 40,
                cols: 140,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("open pty");
        let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_srig"));
        command.args(["tui", "--db", database.to_str().expect("database path")]);
        if import_project {
            command.args([
                "--project",
                fixture.to_str().expect("fixture path"),
                "--import",
            ]);
        }
        let child = pair.slave.spawn_command(command).expect("spawn tui");
        let master = pair.master;
        let mut writer = master.take_writer().expect("take writer");
        let reader = master.try_clone_reader().expect("clone reader");
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || read_output(reader, sender));
        writer.flush().expect("flush terminal writer");
        Self {
            fixture,
            master,
            child,
            writer,
            receiver,
            parser: Parser::new(40, 140, 0),
            transcript: Vec::new(),
        }
    }

    /// Sends raw terminal input to the running TUI.
    ///
    /// # Parameters
    /// - `input`: Terminal bytes to send to the child process.
    ///
    /// # Panics
    /// Panics when the PTY input stream cannot be written.
    pub fn send(&mut self, input: &[u8]) {
        self.writer.write_all(input).expect("send terminal input");
        self.writer.flush().expect("flush terminal input");
    }

    /// Reads the PTY until a visible screen string appears or a timeout expires.
    ///
    /// # Parameters
    /// - `expected`: Visible text that must appear on the virtual screen.
    ///
    /// # Returns
    /// `true` when the text appears before the timeout.
    pub fn read_until(&mut self, expected: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if !self.receive_frame(remaining) {
                break;
            }
            if self.screen().contains(expected) {
                return true;
            }
        }
        false
    }

    /// Reads a fresh PTY frame until a visible string disappears.
    ///
    /// # Parameters
    /// - `unexpected`: Visible text that must disappear from the screen.
    ///
    /// # Returns
    /// `true` when the text disappears before the timeout.
    pub fn read_until_not(&mut self, unexpected: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if !self.receive_frame(remaining) {
                break;
            }
            if !self.screen().contains(unexpected) {
                return true;
            }
        }
        false
    }

    /// Reads and applies one PTY frame, including terminal query responses.
    ///
    /// # Parameters
    /// - `timeout`: Maximum time to wait for a PTY frame.
    ///
    /// # Returns
    /// `true` when a frame was received.
    fn receive_frame(&mut self, timeout: Duration) -> bool {
        let Ok(bytes) = self.receiver.recv_timeout(timeout) else {
            return false;
        };
        self.transcript.extend_from_slice(&bytes);
        if bytes.windows(4).any(|window| window == b"\x1b[6n") {
            self.send(b"\x1b[1;1R");
        }
        self.parser.process(&bytes);
        true
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

    /// Reads the golden source file from the process-owned fixture.
    ///
    /// # Returns
    /// Current source bytes decoded as UTF-8.
    ///
    /// # Panics
    /// Panics when the fixture source cannot be read.
    #[must_use]
    pub fn source(&self) -> String {
        fs::read_to_string(self.fixture.join("src").join("lib.rs")).expect("read source")
    }

    /// Adds an external marker to the live Rust source.
    ///
    /// # Panics
    /// Panics when the fixture source cannot be read or written.
    pub fn external_edit(&self) {
        let path = self.fixture.join("src").join("lib.rs");
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
