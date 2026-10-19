//! Provides small interaction helpers for the PTY driver.

use super::TuiPty;
use std::time::{Duration, Instant};

impl TuiPty {
    /// Reads a fresh frame after an interaction before checking contents.
    ///
    /// - `expected`: Visible text expected after the interaction.
    ///
    /// `true` when the text appears before the timeout.
    /// # Returns
    /// The operation result.
    pub fn read_until_next(&mut self, expected: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if !self.receive_frame(remaining) {
                return false;
            }
            if self.screen().contains(expected) {
                return true;
            }
        }
        false
    }
}
