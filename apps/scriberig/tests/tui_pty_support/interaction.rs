//! Provides small interaction helpers for the PTY driver.

use super::TuiPty;

impl TuiPty {
    /// Types a palette query and waits until the query is visible in the native UI.
    ///
    /// # Parameters
    /// - `query`: Exact palette command or search text.
    ///
    /// # Returns
    /// Unit when the complete query is rendered.
    ///
    /// # Errors
    /// Returns the terminal diagnostic when the query does not appear in time.
    pub fn type_palette_query(&mut self, query: &str) -> Result<(), String> {
        self.send(query.as_bytes());
        if self.read_until(&format!("Search: {query}")) {
            Ok(())
        } else {
            Err(self.diagnostic())
        }
    }

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
