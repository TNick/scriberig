//! Builds an isolated copied checkout for the self-hosting product test.

use std::path::PathBuf;

use super::{TuiPty, tui_pty_fixture};

impl TuiPty {
    /// Starts a fresh native TUI in a copied Scriberig checkout for self-hosting.
    ///
    /// # Returns
    /// A no-project TUI process with the deterministic PHP scenario selected.
    ///
    /// # Panics
    /// Panics when the checkout copy, Git index, or PTY process cannot be prepared.
    #[must_use]
    #[allow(dead_code)]
    pub fn start_self_hosting() -> Self {
        let fixture = tui_pty_fixture::temporary_fixture();
        let checkout = fixture.join("scriberig");
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        tui_pty_fixture::copy_repository_checkout(&repository, &checkout);
        let git = |arguments: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(&checkout)
                    .args(arguments)
                    .status()
                    .expect("git executable")
                    .success(),
                "Git command failed: {arguments:?}"
            );
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        Self::start_configured_scenario(
            fixture,
            None,
            false,
            Some(checkout),
            true,
            Some("self_hosting_php"),
        )
    }
}
