//! Defines optional project behavior selected for TUI startup.

use std::path::PathBuf;

/// Describes optional project behavior selected for TUI startup.
///
/// # Public members
/// None.
///
/// # Private members
/// - `path`: Optional filesystem project passed to the product CLI.
/// - `import`: Whether startup imports the selected project.
/// - `git_tracked`: Whether import is restricted to Git-indexed files.
pub(super) struct ProjectStartupOptions {
    /// Optional filesystem project passed to the product CLI.
    pub(super) path: Option<PathBuf>,
    /// Whether startup imports the selected project.
    pub(super) import: bool,
    /// Whether import is restricted to Git-indexed files.
    pub(super) git_tracked: bool,
}
