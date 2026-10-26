//! Starts a TUI process with a persisted upstream model-property schema.

use super::{TuiPty, startup_options::ProjectStartupOptions, tui_pty_fixture::temporary_fixture};

impl TuiPty {
    /// Starts a deterministic TUI with a writable upstream model configuration.
    ///
    /// - `model_config`: Upstream `ModelsConfig` JSON written to the fixture.
    ///
    /// # Returns
    /// The running TUI process with its model configuration path set.
    ///
    /// # Panics
    /// Panics when fixture files or the TUI process cannot be created.
    #[must_use]
    pub fn start_with_model_config(model_config: &str) -> Self {
        let fixture = temporary_fixture();
        let checkout = super::tui_pty_fixture::bootstrap_project(&fixture);
        let config_path = fixture.join("models.json");
        std::fs::write(&config_path, model_config).expect("write upstream model configuration");
        let environment = [(
            "SCRIBE_AGENT_MODEL_CONFIG_PATH",
            config_path.display().to_string(),
        )];
        TuiPty::start_configured_scenario_with_catalog_and_environment(
            fixture,
            ProjectStartupOptions {
                path: None,
                import: false,
                git_tracked: false,
            },
            Some(checkout),
            true,
            Some("bootstrap"),
            None,
            None,
            &environment,
        )
    }
}
