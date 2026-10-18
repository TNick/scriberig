//! Implements simple TUI process startup helpers.

use super::TuiPty;

impl TuiPty {
    /// Starts the native TUI with an empty database and no selected project.
    ///
    /// # Returns
    /// A running TUI session with a fresh database and no project arguments.
    ///
    /// # Panics
    /// Panics when the pseudo-terminal or child process cannot be created.
    #[must_use]
    pub fn start_empty() -> Self {
        Self::start_with_project(super::tui_pty_fixture::temporary_fixture(), false)
    }
}
