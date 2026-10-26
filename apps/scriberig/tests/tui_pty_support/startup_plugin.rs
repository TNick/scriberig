//! Starts a PTY process with an explicitly configured isolated provider package.

use super::{TuiPty, startup_options::ProjectStartupOptions};

impl TuiPty {
    /// Starts the TUI with the isolated fixture provider package enabled.
    ///
    /// The package is copied beside the temporary project and activated only
    /// through explicit provider/command allowlists and the product worker.
    ///
    /// # Panics
    /// Panics when the fixture artifact cannot be located or copied, or the
    /// terminal process cannot be started.
    ///
    /// # Returns
    /// The product process started with the isolated provider fixture.
    #[must_use]
    pub fn start_with_provider_package() -> Self {
        let fixture = super::tui_pty_fixture::temporary_fixture();
        let extension_dir = fixture.join("extensions");
        std::fs::create_dir(&extension_dir).expect("create extension fixture");
        let executable = std::env::current_exe().expect("PTY test executable path");
        let deps = executable.parent().expect("PTY test deps directory");
        let filename = if cfg!(target_os = "macos") {
            "libscriberig_test_provider_plugin.dylib"
        } else if cfg!(windows) {
            "scriberig_test_provider_plugin.dll"
        } else {
            "libscriberig_test_provider_plugin.so"
        };
        let artifact = [
            deps.join(filename),
            deps.parent().expect("profile directory").join(filename),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .expect("provider fixture cdylib beside PTY test executable");
        std::fs::copy(&artifact, extension_dir.join(filename)).expect("stage provider fixture");
        let env = [
            (
                "SCRIBE_AGENT_EXTENSION_DIRS",
                extension_dir.display().to_string(),
            ),
            (
                "SCRIBE_AGENT_PLUGIN_WORKER",
                env!("CARGO_BIN_EXE_srig").to_owned(),
            ),
            (
                "SCRIBE_AGENT_PLUGIN_PROVIDERS",
                "scriberig-fixture-provider".to_owned(),
            ),
            (
                "SCRIBE_AGENT_PLUGIN_COMMANDS",
                "scriberig-fixture-command".to_owned(),
            ),
            (
                "SCRIBE_AGENT_PLUGIN_TOOLS",
                "scriberig_fixture_tool".to_owned(),
            ),
            ("SCRIBE_AGENT_PLUGIN_CAPABILITIES", String::new()),
            (
                "SCRIBE_AGENT_PLUGIN_CAPABILITY",
                "fixture-provider".to_owned(),
            ),
            ("SCRIBE_AGENT_PROVIDER", "openai_compatible".to_owned()),
        ];
        Self::start_configured_scenario_with_catalog_and_environment(
            fixture.clone(),
            ProjectStartupOptions {
                path: None,
                import: false,
                git_tracked: false,
            },
            Some(fixture),
            false,
            None,
            None,
            None,
            &env,
        )
    }
}
