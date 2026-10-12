//! Provides small interaction helpers for the PTY driver.

use super::TuiPty;

impl TuiPty {
    /// Reads a fresh frame after an interaction before checking contents.
    ///
    /// # Parameters
    /// - `expected`: Visible text expected after the interaction.
    ///
    /// # Returns
    /// `true` when the text appears before the timeout.
    pub fn read_until_next(&mut self, expected: &str) -> bool {
        self.read_until(expected)
    }
}
