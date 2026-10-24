//! Drives keyboard input and waits for observable native TUI frames.

use std::io::Write;
use std::time::{Duration, Instant};

use scriberig_core::EntityId;
use scriberig_session::SessionStore;
use scriberig_session_sqlite::SqliteSessionStore;

use super::TuiPty;

impl TuiPty {
    /// Loads one durable session while retrying transient SQLite contention.
    ///
    /// - `self`: Active PTY owning the fixture database.
    /// - `session`: Exact session identity to load.
    ///
    /// # Returns
    /// The persisted session value.
    ///
    /// # Panics
    /// Panics when the session is absent, another database error occurs, or the
    /// session remains unavailable for 120 seconds.
    #[must_use]
    pub fn load_session(&self, session: EntityId) -> scriberig_session::Session {
        let database = self.fixture_path().join("scriberig.sqlite");
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let store = match SqliteSessionStore::open(&database) {
                Ok(store) => store,
                Err(error) if error.to_string().contains("database is locked") => {
                    assert!(
                        Instant::now() < deadline,
                        "PTY session database stayed locked"
                    );
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(error) => panic!("open PTY session store: {error}"),
            };
            let loaded = store.load(session);
            drop(store);
            match loaded {
                Ok(Some(saved)) => return saved,
                Ok(None) => {}
                Err(error) if error.contains("database is locked") => {}
                Err(error) => panic!("read PTY session: {error}"),
            }
            assert!(Instant::now() < deadline, "PTY session was not readable");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Reads the sole durable session identity created by this fresh fixture.
    ///
    /// The TUI deliberately keeps identifiers out of ordinary conversation
    /// rendering, so integration assertions read the shared durable store.
    ///
    /// `self`: Active PTY and its private fixture database.
    ///
    /// The stored session identity.
    /// # Returns
    /// The operation result.
    /// # Panics
    /// Panics when the fixture does not create exactly one session in time.
    #[must_use]
    pub fn stored_session_id(&self) -> EntityId {
        let database = self.fixture_path().join("scriberig.sqlite");
        let deadline = Instant::now() + Duration::from_secs(20);
        let store = loop {
            match SqliteSessionStore::open(&database) {
                Ok(store) => break store,
                Err(error) if error.to_string().contains("database is locked") => {
                    assert!(
                        Instant::now() < deadline,
                        "PTY session database stayed locked"
                    );
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(error) => panic!("open PTY session store: {error}"),
            }
        };
        loop {
            let sessions = match store.list() {
                Ok(sessions) => sessions,
                Err(error) if error.contains("database is locked") => Vec::new(),
                Err(error) => panic!("list PTY sessions: {error}"),
            };
            if sessions.len() == 1 {
                return sessions[0].id;
            }
            assert!(
                sessions.is_empty() && Instant::now() < deadline,
                "expected one durable session, got {}",
                sessions.len()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Sends raw terminal input to the running TUI.
    ///
    /// - `input`: Terminal bytes to send to the child process.
    ///
    /// # Panics
    /// Panics when the PTY input stream cannot be written.
    pub fn send(&mut self, input: &[u8]) {
        self.writer.write_all(input).expect("send terminal input");
        self.writer.flush().expect("flush terminal input");
    }

    /// Types a command incrementally so slash completion receives ordered keys.
    ///
    /// - `input`: Terminal bytes to type into the child process.
    ///
    /// # Panics
    /// Panics when the PTY input stream cannot be written.
    pub fn send_slow(&mut self, input: &[u8]) {
        for byte in input {
            if *byte == b'\r' {
                std::thread::sleep(Duration::from_millis(300));
            }
            self.send(std::slice::from_ref(byte));
            std::thread::sleep(Duration::from_millis(12));
        }
    }

    /// Reads the PTY until a visible screen string appears or a timeout expires.
    ///
    /// - `expected`: Visible text that must appear on the virtual screen.
    ///
    /// `true` when the text appears before the timeout.
    /// # Returns
    /// The operation result.
    pub fn read_until(&mut self, expected: &str) -> bool {
        self.read_until_for(expected, Duration::from_secs(20))
    }

    /// Reads the PTY until visible text appears or the requested timeout expires.
    ///
    /// - `expected`: Visible text that must appear on the virtual screen.
    /// - `timeout`: Maximum wait duration.
    ///
    /// `true` when the text appears before timeout.
    /// # Returns
    /// The operation result.
    pub fn read_until_for(&mut self, expected: &str, timeout: Duration) -> bool {
        if self.screen().contains(expected) {
            return true;
        }
        let deadline = Instant::now() + timeout;
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

    /// Reads PTY frames until the child stops emitting output or a bound expires.
    ///
    /// `read_until` stops at the first generated frame, so an interaction that
    /// redraws several times can be observed before the terminal is current. This
    /// wait bounds both the quiet window between frames and the whole interaction.
    ///
    /// - `quiet`: Inactivity gap that counts as a settled frame stream.
    /// - `timeout`: Maximum total wait for the interaction to settle.
    ///
    /// `true` when the frame stream went quiet before `timeout` elapsed.
    /// # Returns
    /// Whether the terminal settled within the bound.
    pub fn settle_for(&mut self, quiet: Duration, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if !self.receive_frame(quiet) {
                return true;
            }
        }
        false
    }

    /// Settles one interaction with the default quiet window and bound.
    ///
    /// `self`: Active PTY whose frame stream is drained.
    ///
    /// `true` when the frame stream settled within the default bound.
    /// # Returns
    /// Whether the terminal settled.
    pub fn settle(&mut self) -> bool {
        self.settle_for(Duration::from_millis(250), Duration::from_secs(5))
    }

    /// Reads a fresh PTY frame until a visible string disappears.
    ///
    /// - `unexpected`: Visible text that must disappear from the screen.
    ///
    /// `true` when the text disappears before the timeout.
    /// # Returns
    /// The operation result.
    pub fn read_until_not(&mut self, unexpected: &str) -> bool {
        if !self.screen().contains(unexpected) {
            return true;
        }
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
    /// - `timeout`: Maximum time to wait for a PTY frame.
    ///
    /// `true` when a frame was received.
    /// # Returns
    /// The operation result.
    pub(super) fn receive_frame(&mut self, timeout: Duration) -> bool {
        let Ok(bytes) = self.receiver.recv_timeout(timeout) else {
            return false;
        };
        self.transcript.extend_from_slice(&bytes);
        let pending = &self.transcript[self.query_cursor..];
        if let Some(position) = pending.windows(4).position(|window| window == b"\x1b[6n") {
            self.query_cursor += position + 4;
            self.send(b"\x1b[1;1R");
        }
        self.parser.process(&bytes);
        true
    }
}
