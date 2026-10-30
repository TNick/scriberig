//! Starts the native TUI against the shared mixed rich-project fixture.

use std::path::PathBuf;

use super::{
    TuiPty, startup_options::ProjectStartupOptions, tui_pty_fixture::copy_directory,
    tui_pty_fixture::temporary_fixture,
};

impl TuiPty {
    /// Starts the TUI after importing an isolated copy of the mixed rich fixture.
    ///
    /// # Returns
    /// A running TUI process with XLSX, DOCX, DXF, QGIS, and `GeoPackage` semantics.
    ///
    /// # Panics
    /// Panics when the fixture cannot be copied or the child process cannot start.
    #[must_use]
    pub fn start_rich_golden() -> Self {
        let fixture = temporary_fixture();
        let project = fixture.join("project");
        let rich_project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("rich-project");
        copy_directory(&rich_project, &project);
        Self::start_configured_scenario(
            fixture,
            ProjectStartupOptions {
                path: Some(project.clone()),
                import: true,
                git_tracked: false,
            },
            Some(project),
            false,
            None,
            None,
        )
    }
}
