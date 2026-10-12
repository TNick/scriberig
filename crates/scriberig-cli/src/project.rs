//! Implements local project catalog and filesystem scan commands.

use std::path::{Path, PathBuf};

use scriberig_core::ProjectId;
use scriberig_store_sqlite::{ProjectRecord, ProjectStatus, SqliteStore};

use clap::{Args, Subcommand};

#[path = "project_import.rs"]
mod project_import;
use project_import::{import_files, scan_files};

/// Selects a project catalog operation.
#[derive(Debug, Args)]
pub struct ProjectArgs {
    /// Selected project operation.
    #[command(subcommand)]
    pub command: ProjectCommand,
}

/// Project catalog and onboarding commands.
#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    /// List projects in the selected database.
    List(ProjectDatabaseArgs),
    /// Scan and register a filesystem project.
    Scan(ProjectScanArgs),
    /// Import the latest filesystem scan into an immutable project snapshot.
    Import(ProjectImportArgs),
    /// Read one project's onboarding status.
    Status(ProjectSelectorArgs),
    /// Change a project's registered filesystem path.
    Relink(ProjectRelinkArgs),
}

/// Selects a project in a local database.
#[derive(Debug, Args)]
pub struct ProjectSelectorArgs {
    /// Stable project identity.
    pub project: scriberig_core::ProjectId,
    /// SQLite database path.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: PathBuf,
    /// Render machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

/// Configures a project database operation.
#[derive(Debug, Args)]
pub struct ProjectDatabaseArgs {
    /// SQLite database path.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: PathBuf,
    /// Render machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

/// Configures an initial project scan.
#[derive(Debug, Args)]
pub struct ProjectScanArgs {
    /// Filesystem path to register and scan.
    pub path: PathBuf,
    /// Optional display name for a newly registered project.
    #[arg(long)]
    pub name: Option<String>,
    /// SQLite database path.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: PathBuf,
    /// Render machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

/// Configures a project import operation.
#[derive(Debug, Args)]
pub struct ProjectImportArgs {
    /// Stable project identity.
    pub project: ProjectId,
    /// SQLite database path.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: PathBuf,
    /// Render machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

/// Configures a project path relink operation.
#[derive(Debug, Args)]
pub struct ProjectRelinkArgs {
    /// Stable project identity.
    pub project: scriberig_core::ProjectId,
    /// New filesystem path.
    #[arg(long)]
    pub path: PathBuf,
    /// SQLite database path.
    #[arg(long, default_value = "playground/dev-app-runtime/scriberig.sqlite")]
    pub db: PathBuf,
    /// Render machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

/// Runs one project command against its selected SQLite database.
///
/// # Parameters
/// - `args`: Parsed project command.
///
/// # Returns
/// A rendered command response.
///
/// # Errors
/// Returns a database, filesystem, or project lookup error.
pub fn run(args: &ProjectArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match &args.command {
        ProjectCommand::List(args) => {
            let store = SqliteStore::open(&args.db)?;
            let projects = store.list_projects()?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&project_json(&projects))?
                );
            } else if projects.is_empty() {
                println!("no projects");
            } else {
                for project in projects {
                    println!(
                        "{}\t{}\t{}\t{:?}",
                        project.id,
                        project.name,
                        project.path.display(),
                        project.status
                    );
                }
            }
        }
        ProjectCommand::Scan(args) => {
            let store = SqliteStore::open(&args.db)?;
            let name = args
                .name
                .clone()
                .or_else(|| path_name(&args.path))
                .unwrap_or_else(|| "project".to_owned());
            let project = store.register_project(name, &args.path)?;
            let manifest = scan(&project.path)?;
            store.set_project_status(project.id, ProjectStatus::ScanReady, None)?;
            let project = store
                .project(project.id)?
                .ok_or_else(|| format!("project {} was not found", project.id))?;
            let response = serde_json::json!({
                "project": project_json(std::slice::from_ref(&project))[0],
                "files": manifest,
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!("project {} scanned", project.id);
                println!("path: {}", project.path.display());
                println!(
                    "files: {}",
                    response["files"].as_array().map_or(0, Vec::len)
                );
            }
        }
        ProjectCommand::Import(args) => {
            run_import(args)?;
        }
        ProjectCommand::Status(args) => {
            let store = SqliteStore::open(&args.db)?;
            let project = store
                .project(args.project)?
                .ok_or_else(|| format!("project {} was not found", args.project))?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&project_json(&[project]))?
                );
            } else {
                print_project(&project);
            }
        }
        ProjectCommand::Relink(args) => {
            let store = SqliteStore::open(&args.db)?;
            store.relink_project(args.project, &args.path)?;
            let project = store
                .project(args.project)?
                .ok_or_else(|| format!("project {} was not found", args.project))?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&project_json(&[project]))?
                );
            } else {
                println!(
                    "project {} relinked to {}",
                    project.id,
                    project.path.display()
                );
            }
        }
    }
    Ok(())
}

/// Runs one project import and renders its result.
///
/// # Parameters
/// - `args`: Import command options.
///
/// # Returns
/// Unit after import output is rendered.
///
/// # Errors
/// Returns a database, filesystem, validation, or promotion error.
fn run_import(args: &ProjectImportArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let store = SqliteStore::open(&args.db)?;
    let project = store
        .project(args.project)?
        .ok_or_else(|| format!("project {} was not found", args.project))?;
    let files = scan_files(&project.path)?;
    let imported = import_files(&store, &project, &files)?;
    let project = store
        .project(args.project)?
        .ok_or_else(|| format!("project {} was not found", args.project))?;
    let response = serde_json::json!({
        "project": project_json(std::slice::from_ref(&project))[0],
        "files": imported.0,
        "snapshot": imported.1.to_string(),
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "project {} imported {} files into {}",
            args.project, imported.0, imported.1
        );
    }
    Ok(())
}

/// Scans supported and unknown files under one project path.
///
/// # Parameters
/// - `path`: Project directory to scan.
///
/// # Returns
/// Deterministically ordered file manifest entries.
///
/// # Errors
/// Returns a filesystem traversal or read error.
fn scan(path: &Path) -> std::io::Result<Vec<serde_json::Value>> {
    Ok(scan_files(path)?
        .into_iter()
        .map(|file| file.json())
        .collect())
}

/// Derives a default project name from a path.
///
/// # Parameters
/// - `path`: Project path.
///
/// # Returns
/// The final path component when it is valid UTF-8.
fn path_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
}

/// Renders project records as stable JSON values.
///
/// # Parameters
/// - `projects`: Project records to render.
///
/// # Returns
/// JSON values containing only public project fields.
fn project_json(projects: &[ProjectRecord]) -> Vec<serde_json::Value> {
    projects
        .iter()
        .map(|project| {
            serde_json::json!({
                "id": project.id.to_string(),
                "name": project.name,
                "path": project.path,
                "status": status_name(&project.status),
                "snapshot": project.current_snapshot.map(|value| value.to_string()),
            })
        })
        .collect()
}

/// Converts a project status into its stable public name.
///
/// # Parameters
/// - `status`: Status to render.
///
/// # Returns
/// Stable status text.
fn status_name(status: &ProjectStatus) -> &'static str {
    match status {
        ProjectStatus::Registered => "registered",
        ProjectStatus::ScanReady => "scan_ready",
        ProjectStatus::Imported => "imported",
        ProjectStatus::Failed => "failed",
    }
}

/// Prints one project in a human-readable form.
///
/// # Parameters
/// - `project`: Project record to print.
fn print_project(project: &ProjectRecord) {
    println!("id: {}", project.id);
    println!("name: {}", project.name);
    println!("path: {}", project.path.display());
    println!("status: {}", status_name(&project.status));
    if let Some(snapshot) = project.current_snapshot {
        println!("snapshot: {snapshot}");
    }
}
