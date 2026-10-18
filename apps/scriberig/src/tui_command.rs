//! Composes local and remote TUI startup with project context.

use std::sync::Arc;

use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, BootstrapEnvironment,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store_sqlite::SqliteStore;

/// Identifies the project context selected for TUI startup.
type ProjectStartup = (ProjectId, Option<EntityId>, Option<SnapshotId>);

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
#[allow(clippy::too_many_lines)]
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
            |path| -> Result<ProjectStartup, Box<dyn std::error::Error + Send + Sync>> {
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("project");
                let service = scriberig_project::ProjectService::new(Arc::clone(&store));
                let report = service.scan_with_options(
                    Some(name.to_owned()),
                    path,
                    scriberig_project::ScanOptions {
                        git_tracked: args.git_tracked,
                    },
                )?;
                let project = report.project;
                if args.import_project {
                    let imported = service.import(project.id)?;
                    let entities = store.project_entities(project.id)?;
                    let entity = entities
                        .iter()
                        .find(|record| record.name.as_deref() == Some("add"))
                        .or_else(|| {
                            entities.iter().find(|record| {
                                record.kind == "source" && record.relative_path == "src/lib.rs"
                            })
                        })
                        .or_else(|| entities.iter().find(|record| record.kind == "source"))
                        .map(|record| record.entity);
                    return Ok((project.id, entity, Some(imported.snapshot)));
                }
                let workspace = project
                    .current_snapshot
                    .map(|_| service.workspace(project.id))
                    .transpose()?;
                Ok((
                    project.id,
                    workspace.as_ref().map(|value| value.root_entity),
                    workspace.map(|value| value.snapshot),
                ))
            },
        )
        .transpose()?;
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let session_store = Arc::new(scriberig_session_sqlite::SqliteSessionStore::open(
        &args.db,
    )?);
    let agent_runner =
        super::native_tui_agent::NativeTuiAgentRunner::from_environment(Arc::clone(&store))?;
    let service = ApplicationService::new(backend)
        .with_session_store(session_store)
        .with_model_library(agent_runner.model_library());
    let labels = project
        .as_ref()
        .map(|(project_id, _, _)| project_labels(&store, *project_id))
        .transpose()?;
    let projection =
        project
            .as_ref()
            .zip(labels.as_ref())
            .and_then(|((project_id, _, snapshot), _)| {
                snapshot.map(|snapshot| {
                    let files = store
                        .project_files(*project_id)
                        .unwrap_or_default()
                        .into_iter()
                        .collect::<Vec<_>>();
                    let by_path = files
                        .iter()
                        .map(|file| (file.relative_path.as_str(), file))
                        .collect::<std::collections::HashMap<_, _>>();
                    let sources = store
                        .project_entities(*project_id)
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|entity| {
                            by_path.get(entity.relative_path.as_str()).map(|file| {
                                (entity.entity, (entity.relative_path, file.bytes.clone()))
                            })
                        })
                        .collect();
                    scriberig_tui::ProjectProjection {
                        project: *project_id,
                        workflow: scriberig_project::ProjectService::new(Arc::clone(&store)),
                        snapshot,
                        sources,
                    }
                })
            });
    let service = scriberig_tui::ApplicationOperationAdapter::new(service)
        .with_agent_runner(agent_runner)
        .with_explorer_labels(labels.unwrap_or_default())
        .with_project_workflow(scriberig_project::ProjectService::new(Arc::clone(&store)));
    let service = if let Some(value) = projection {
        service.with_project_projection(value)
    } else {
        service
    };
    let context = ApplicationContext {
        capabilities: scriberig_app::CapabilityContext {
            granted: vec![scriberig_app::CapabilityId::new("inspect.entity")?],
            grants: vec![scriberig_app::Capability {
                scope: scriberig_app::CapabilityScope::Project,
                actions: super::agent_actions(),
                constraints: scriberig_app::CapabilityConstraints::unrestricted(),
            }],
        },
        principal: Some(scriberig_app::Principal::new("local-tui-agent")),
        ..ApplicationContext::empty().with_bootstrap(BootstrapEnvironment {
            working_directory: std::env::current_dir()?,
        })
    }
    .with_user_preferences_principal("local-user");
    scriberig_tui::run(
        &service,
        &context,
        &scriberig_tui::TuiConfig {
            interactive: true,
            project: project.as_ref().map(|value| {
                format!(
                    "{} @ {}",
                    args.project
                        .as_ref()
                        .map_or_else(|| value.0.to_string(), |path| path.display().to_string()),
                    value
                        .2
                        .map_or_else(|| "unimported".to_owned(), |snapshot| snapshot.to_string())
                )
            }),
            initial_entity: project.as_ref().and_then(|value| value.1),
            snapshot: project.as_ref().and_then(|value| value.2),
        },
    )?;
    Ok(())
}

/// Builds readable explorer labels from the imported project ownership manifest.
///
/// # Parameters
/// - `store`: SQLite store containing project ownership records.
/// - `project`: Project whose entities are being displayed.
///
/// # Returns
/// A stable entity-to-label map for local TUI navigation.
///
/// # Errors
/// Returns a SQLite error when the ownership manifest cannot be read.
fn project_labels(
    store: &Arc<SqliteStore>,
    project: ProjectId,
) -> Result<scriberig_tui::ExplorerLabels, Box<dyn std::error::Error + Send + Sync>> {
    let records = store.project_entities(project)?;
    let mut counts = std::collections::HashMap::<(String, String), usize>::new();
    let mut labels = scriberig_tui::ExplorerLabels::new();
    for record in records {
        if record.kind == "source" {
            labels.insert(record.entity, record.relative_path);
            continue;
        }
        if let Some(name) = record.name {
            labels.insert(record.entity, format!("{name} · {}", record.kind));
            continue;
        }
        let key = (record.relative_path.clone(), record.kind.clone());
        let ordinal = counts.entry(key).or_insert(0);
        *ordinal += 1;
        labels.insert(
            record.entity,
            format!("{} :: {} #{}", record.relative_path, record.kind, ordinal),
        );
    }
    Ok(labels)
}
