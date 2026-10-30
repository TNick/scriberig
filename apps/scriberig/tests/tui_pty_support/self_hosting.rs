//! Builds an isolated copied checkout for the self-hosting product test.

use std::path::PathBuf;

use super::{TuiPty, tui_pty_fixture};

impl TuiPty {
    /// Starts a copied Scriberig checkout with a deterministic import failure after manifest review.
    ///
    /// A no-project TUI process prepared to verify failed onboarding remains inactive.
    ///
    /// # Panics
    /// Panics when the checkout copy, Git index, or PTY process cannot be prepared.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn start_bootstrap_import_failure() -> Self {
        Self::start_self_hosting_scenario("bootstrap_import_failure")
    }

    /// Starts a fresh native TUI in a copied Scriberig checkout for self-hosting.
    ///
    /// A no-project TUI process with the deterministic PHP scenario selected.
    ///
    /// # Panics
    /// Panics when the checkout copy, Git index, or PTY process cannot be prepared.
    #[must_use]
    #[allow(dead_code)]
    /// # Returns
    /// The operation result.
    pub fn start_self_hosting() -> Self {
        Self::start_self_hosting_scenario("self_hosting_php")
    }

    /// Starts a copied-checkout TUI with a deterministic PHP validation failure.
    ///
    /// A no-project TUI process whose PHP candidate intentionally fails validation.
    ///
    /// # Panics
    /// Panics when the checkout copy, Git index, or PTY process cannot be prepared.
    #[must_use]
    #[allow(dead_code)]
    /// # Returns
    /// The operation result.
    pub fn start_self_hosting_validation_failure() -> Self {
        Self::start_self_hosting_scenario("self_hosting_php_validation_failure")
    }

    /// Starts a copied-checkout TUI that repairs a failed PHP candidate by normal reproposal.
    ///
    /// A no-project TUI process with a deterministic candidate repair scenario.
    ///
    /// # Panics
    /// Panics when the checkout copy, Git index, or PTY process cannot be prepared.
    #[must_use]
    #[allow(dead_code)]
    /// # Returns
    /// The operation result.
    pub fn start_self_hosting_validation_repair() -> Self {
        Self::start_self_hosting_scenario("self_hosting_php_validation_repair")
    }

    /// Starts a copied-checkout TUI whose first onboarding provider response is delayed.
    ///
    /// A no-project TUI process prepared for in-flight cancellation.
    ///
    /// # Panics
    /// Panics when the checkout copy, Git index, or PTY process cannot be prepared.
    #[must_use]
    #[allow(dead_code)]
    /// # Returns
    /// The operation result.
    pub fn start_self_hosting_cancellation() -> Self {
        Self::start_self_hosting_scenario("self_hosting_php_cancellation")
    }

    /// Starts a self-hosting scenario delayed during architecture discovery.
    ///
    /// A no-project TUI process prepared to cancel a read-only discovery child.
    #[must_use]
    #[allow(dead_code)]
    /// # Returns
    /// The operation result.
    pub fn start_self_hosting_discovery_cancellation() -> Self {
        Self::start_self_hosting_scenario("self_hosting_php_discovery_cancellation")
    }

    /// Starts a self-hosting scenario delayed before implementation reinspection.
    ///
    /// A no-project TUI process prepared to cancel the implementation child.
    #[must_use]
    #[allow(dead_code)]
    /// # Returns
    /// The operation result.
    pub fn start_self_hosting_implementation_cancellation() -> Self {
        Self::start_self_hosting_scenario("self_hosting_php_implementation_cancellation")
    }

    /// Starts a copied-checkout TUI with one scripted self-hosting provider scenario.
    ///
    /// - `scenario`: Registered deterministic self-hosting scenario.
    /// - `super`: Parent TUI support module.
    ///
    /// A no-project TUI process using the selected deterministic scenario.
    ///
    /// # Panics
    /// Panics when the checkout copy, Git index, or PTY process cannot be prepared.
    /// # Returns
    /// The operation result.
    fn start_self_hosting_scenario(scenario: &str) -> Self {
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
        let mut staged_paths = vec![
            "add",
            "--",
            "Cargo.toml",
            "Cargo.lock",
            "README.md",
            ":(glob)**/Cargo.toml",
            "crates",
            "apps",
            "tests/fixtures",
            "tests/conformance/fixtures",
            "tests/self-hosting/php-extension",
        ];
        if checkout.join(".serena/project.yml").is_file() {
            staged_paths.push(".serena/project.yml");
        }
        git(&staged_paths);
        git(&["clean", "-fdx"]);
        Self::start_configured_scenario(
            fixture,
            super::startup_options::ProjectStartupOptions {
                path: None,
                import: false,
                git_tracked: false,
            },
            Some(checkout),
            true,
            Some(scenario),
            None,
        )
    }

    /// Restarts the copied checkout's candidate-built executable and imports its PHP sources.
    ///
    /// - `fixture`: Retained PTY fixture containing the database and checkout.
    /// - `checkout`: Candidate-applied Scriberig checkout.
    /// - `executable`: Candidate-built native executable.
    ///
    /// A fresh real TUI process using the candidate's compiled domain registry.
    ///
    /// # Panics
    /// Panics when the PTY or child process cannot be prepared.
    #[must_use]
    #[allow(dead_code)]
    /// # Returns
    /// The operation result.
    pub fn restart_candidate_checkout(
        fixture: PathBuf,
        checkout: PathBuf,
        executable: PathBuf,
    ) -> Self {
        Self::start_configured_scenario(
            fixture,
            super::startup_options::ProjectStartupOptions {
                path: Some(checkout.clone()),
                import: true,
                git_tracked: true,
            },
            Some(checkout),
            false,
            None,
            Some(executable),
        )
    }
}
