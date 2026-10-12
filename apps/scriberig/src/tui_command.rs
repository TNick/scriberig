//! Composes local and remote TUI startup with project context.

use std::sync::Arc;

use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, StoreInspectionBackend,
    StoreOperationProvider,
};
use scriberig_store_sqlite::SqliteStore;

/// Starts TUI mode using either a remote endpoint or local project database.
///
/// # Parameters
/// - `args`: Parsed TUI startup settings.
/// - `super`: Parent application module providing local context.
///
/// # Returns
/// Unit after the terminal session ends.
///
/// # Errors
/// Returns a database, transport, or terminal error.
pub(super) fn run(
    args: &scriberig_cli::TuiArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(endpoint) = args.connect.as_ref() {
        let transport = scriberig_client::HttpTransport::from_endpoint(endpoint)?;
        let client = scriberig_client::Client::new(transport);
        let service = scriberig_tui::RemoteAdapter::new(client);
        let context = super::local_read_context("remote-tui");
        scriberig_tui::run(
            &service,
            &context,
            &scriberig_tui::TuiConfig {
                interactive: true,
                ..scriberig_tui::TuiConfig::default()
            },
        )?;
        return Ok(());
    }
    let store = Arc::new(SqliteStore::open(&args.db)?);
    let project = args
        .project
        .as_ref()
        .map(
            |path| -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("project");
                let service = scriberig_project::ProjectService::new(Arc::clone(&store));
                let report = service.scan(Some(name.to_owned()), path)?;
                let project = report.project;
                if args.import_project {
                    service.import(project.id)?;
                }
                Ok(project.id.to_string())
            },
        )
        .transpose()?;
    let fixture = scriberig_server::ensure_fixture(&store).map_err(std::io::Error::other)?;
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let service = ApplicationService::new(backend);
    let service = scriberig_tui::ApplicationOperationAdapter::new(service);
    let context = ApplicationContext::empty();
    scriberig_tui::run(
        &service,
        &context,
        &scriberig_tui::TuiConfig {
            interactive: true,
            initial_entity: Some(fixture.root_entity),
            snapshot: Some(fixture.snapshot),
            project,
        },
    )?;
    Ok(())
}
