//! Drives keyboard input and waits for observable native TUI frames.

use std::io::Write;
use std::time::{Duration, Instant};

use super::TuiPty;

impl TuiPty {
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
        self.read_until_for(expected, Duration::from_secs(20))
    }

    /// Reads the PTY until visible text appears or the requested timeout expires.
    ///
    /// # Parameters
    /// - `expected`: Visible text that must appear on the virtual screen.
    /// - `timeout`: Maximum wait duration.
    ///
    /// # Returns
    /// `true` when the text appears before timeout.
    pub fn read_until_for(&mut self, expected: &str, timeout: Duration) -> bool {
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
}
