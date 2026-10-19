//! Owns bounded lifecycle and cleanup operations for native PTY sessions.

use super::*;

impl TuiPty {
    /// Releases a deterministic cancellation scenario after the UI confirms cancellation.
    ///
    /// - `self`: Active TUI process and owning fixture directory.
    ///
    /// # Panics
    /// Panics when the provider release marker cannot be written.
    #[allow(dead_code)]
    pub fn release_agent_provider(&self) {
        fs::write(self.fixture.join("agent-provider.release"), b"release")
            .expect("release deterministic provider gate");
    }

    /// Requests normal shutdown and bounds cleanup if the process does not exit.
    ///
    /// - `self`: Owned TUI session to shut down and clean up.
    ///
    /// `true` when the child exits successfully.
    ///
    /// # Panics
    /// Panics when the child cannot be polled, stopped, or the fixture removed.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn shutdown(mut self) -> bool {
        self.send(&[3]);
        self.finish()
    }

    /// Stops a session after an expected recoverable operation failure.
    ///
    /// The product assertions are made before teardown; this helper preserves
    /// the failure screen while allowing the terminal child to be collected.
    ///
    /// - `self`: Running PTY session.
    ///
    /// Whether the child reported a successful exit.
    ///
    /// # Panics
    /// Panics when the child cannot be polled, stopped, or the fixture removed.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn shutdown_allow_failure(mut self) -> bool {
        self.send(&[3]);
        let deadline = Instant::now() + Duration::from_secs(3);
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("poll tui") {
                break status;
            }
            if let Ok(bytes) = self.receiver.recv_timeout(Duration::from_millis(25)) {
                self.transcript.extend_from_slice(&bytes);
                if bytes.windows(4).any(|window| window == b"\x1b[6n") {
                    self.writer
                        .write_all(b"\x1b[1;1R")
                        .expect("reply to terminal cursor query");
                    self.writer.flush().expect("flush terminal cursor reply");
                }
                self.parser.process(&bytes);
            }
            if Instant::now() >= deadline {
                self.child.kill().expect("kill tui");
                break self.child.wait().expect("wait for tui");
            }
            thread::sleep(Duration::from_millis(25));
        };
        for _ in 0..20 {
            if fs::remove_dir_all(&self.fixture).is_ok() {
                return status.success();
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = fs::remove_dir_all(&self.fixture);
        status.success()
    }

    /// Waits for a TUI that was asked to quit through another key binding.
    ///
    /// - `self`: Owned TUI session to wait for and clean up.
    ///
    /// `true` when the child exits successfully.
    ///
    /// # Panics
    /// Panics when the child cannot be polled, stopped, or the fixture removed.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn wait_for_exit(mut self) -> bool {
        self.finish()
    }

    /// Waits for the child and removes the temporary fixture.
    ///
    /// `true` when the child exits successfully.
    ///
    /// # Panics
    /// Panics when the child cannot be polled, stopped, or the fixture removed.
    /// # Returns
    /// The operation result.
    fn finish(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(3);
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("poll tui") {
                break status;
            }
            if Instant::now() >= deadline {
                self.child.kill().expect("kill stuck tui");
                break self.child.wait().expect("wait for killed tui");
            }
            thread::sleep(Duration::from_millis(25));
        };
        let transcript = String::from_utf8_lossy(&self.transcript);
        assert!(
            status.success(),
            "TUI exited unsuccessfully: {status:?}; transcript: {transcript}"
        );
        fs::remove_dir_all(&self.fixture).expect("remove fixture");
        status.success()
    }
}

impl Drop for TuiPty {
    /// Cleans up a failed test without leaving a child process behind.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = fs::remove_dir_all(&self.fixture);
    }
}
