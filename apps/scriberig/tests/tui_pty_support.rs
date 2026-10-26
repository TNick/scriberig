//! Provides a native pseudo-terminal driver for TUI integration tests.

use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock, mpsc::Receiver},
    thread,
    time::{Duration, Instant},
};

use portable_pty::PtySize;
use vt100::Parser;

#[path = "tui_pty_support/lifecycle.rs"]
mod lifecycle;

#[allow(dead_code)]
#[path = "tui_pty_support/candidate_review.rs"]
pub mod candidate_review;
#[path = "tui_pty_support/onboarding_wait.rs"]
pub(crate) mod onboarding_wait;
#[path = "tui_pty_support/driver.rs"]
mod tui_pty_driver;
#[path = "tui_pty_support/fixture.rs"]
mod tui_pty_fixture;
#[path = "tui_pty_support/interaction.rs"]
mod tui_pty_interaction;

#[path = "tui_pty_support/self_hosting.rs"]
mod self_hosting;
#[path = "tui_pty_support/startup.rs"]
mod startup;
#[path = "tui_pty_support/startup_arithmetic.rs"]
mod startup_arithmetic;
#[path = "tui_pty_support/startup_model_config.rs"]
mod startup_model_config;
#[path = "tui_pty_support/startup_options.rs"]
mod startup_options;
#[cfg(feature = "provider-package-worker-tests")]
#[path = "tui_pty_support/startup_plugin.rs"]
mod startup_plugin;

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
    query_cursor: usize,
}

/// Returns the Cargo target directory configured for spawned TUI processes.
///
/// The exact workspace target path used by PTY setup and observers.
#[must_use]
/// # Returns
/// The operation result.
pub fn workspace_target_directory() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("target")
        },
        PathBuf::from,
    )
}

impl TuiPty {
    /// Resizes the native terminal surface.
    ///
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
    /// The current screen contents without terminal control sequences.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn screen(&self) -> String {
        self.parser.screen().contents()
    }

    /// Returns the fixture directory used for the database and checkout.
    ///
    /// The temporary fixture root.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn fixture_path(&self) -> &std::path::Path {
        &self.fixture
    }

    /// Reads the golden source file from the process-owned fixture.
    ///
    /// Current source bytes decoded as UTF-8.
    ///
    /// # Panics
    /// Panics when the fixture source cannot be read.
    #[must_use]
    /// # Returns
    /// The operation result.
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
    /// - `self`: Running PTY session.
    ///
    /// The retained fixture directory.
    ///
    /// # Panics
    /// Panics when the child cannot be polled or stopped.
    #[must_use]
    /// # Returns
    /// The operation result.
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
    /// Diagnostic screen and transcript text for a failed assertion.
    #[must_use]
    /// # Returns
    /// The operation result.
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
/// The process-wide PTY test lock guard.
/// # Returns
/// The operation result.
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
