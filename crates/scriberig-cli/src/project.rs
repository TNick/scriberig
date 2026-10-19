//! Implements the project catalog and onboarding commands.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Args, Subcommand};
use scriberig_core::ProjectId;
use scriberig_project::{ProjectService, ScanFile, ScanOptions};
use scriberig_store_sqlite::{ProjectRecord, ProjectStatus, SqliteStore};

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
    /// Import the latest persisted filesystem scan.
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
    pub project: ProjectId,
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
    /// Restrict the persisted manifest to existing Git-indexed files.
    #[arg(long)]
    pub git_tracked: bool,
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
    pub project: ProjectId,
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
/// - `args`: Parsed project command.
///
/// A rendered command response.
///
/// Returns a database, filesystem, or project lookup error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn run(args: &ProjectArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match &args.command {
        ProjectCommand::List(args) => run_list(args)?,
        ProjectCommand::Scan(args) => run_scan(args)?,
        ProjectCommand::Import(args) => run_import(args)?,
        ProjectCommand::Status(args) => run_status(args)?,
        ProjectCommand::Relink(args) => run_relink(args)?,
    }
    Ok(())
}

/// Runs the project list command.
///
/// - `args`: List command options.
///
/// Unit after output is rendered.
///
/// Returns a database or serialization error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn run_list(args: &ProjectDatabaseArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    Ok(())
}

/// Runs a persisted project scan.
///
/// - `args`: Scan command options.
///
/// Unit after output is rendered.
///
/// Returns a filesystem, database, or serialization error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn run_scan(args: &ProjectScanArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let report = service(&args.db)?.scan_with_options(
        args.name.clone(),
        &args.path,
        ScanOptions {
            git_tracked: args.git_tracked,
        },
    )?;
    let response = serde_json::json!({
        "project": project_json(std::slice::from_ref(&report.project))[0],
        "files": report.files.iter().map(file_json).collect::<Vec<_>>(),
        "gitTracked": report.git_tracked,
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("project {} scanned", report.project.id);
        println!("path: {}", report.project.path.display());
        println!("files: {}", report.files.len());
        println!("git-tracked only: {}", report.git_tracked);
    }
    Ok(())
}

/// Runs the semantic project import.
///
/// - `args`: Import command options.
///
/// Unit after output is rendered.
///
/// Returns a project validation, database, or serialization error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn run_import(args: &ProjectImportArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let report = match service(&args.db)?.import(args.project) {
        Ok(report) => report,
        Err(error) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "code": error.code,
                        "message": error.message,
                        "diagnostics": error.diagnostics.iter().map(|diagnostic| serde_json::json!({
                            "path": diagnostic.path,
                            "adapter": diagnostic.adapter,
                            "stage": diagnostic.stage,
                            "code": diagnostic.code,
                            "message": diagnostic.message,
                            "retryable": diagnostic.retryable,
                        })).collect::<Vec<_>>(),
                    }))?
                );
            } else {
                eprintln!("{error}");
                for diagnostic in &error.diagnostics {
                    eprintln!(
                        "{} [{}:{}] {}",
                        diagnostic.path, diagnostic.adapter, diagnostic.code, diagnostic.message
                    );
                }
            }
            return Err(Box::new(error));
        }
    };
    let response = serde_json::json!({
        "project": project_json(std::slice::from_ref(&report.project))[0],
        "files": report.files,
        "entities": report.entities,
        "snapshot": report.snapshot.to_string(),
        "diagnostics": [],
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "project {} imported {} files and {} entities into {}",
            args.project, report.files, report.entities, report.snapshot
        );
    }
    Ok(())
}

/// Reads one project's catalog status.
///
/// - `args`: Status command options.
///
/// Unit after output is rendered.
///
/// Returns a database or project lookup error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn run_status(args: &ProjectSelectorArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    Ok(())
}

/// Relinks a project while preserving its stable identity.
///
/// - `args`: Relink command options.
///
/// Unit after output is rendered.
///
/// Returns a database or project lookup error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn run_relink(args: &ProjectRelinkArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    Ok(())
}

/// Opens the shared project onboarding service.
///
/// - `path`: SQLite database path.
///
/// A shared project service.
///
/// Returns a database initialization error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn service(
    path: &std::path::Path,
) -> Result<ProjectService, Box<dyn std::error::Error + Send + Sync>> {
    Ok(ProjectService::new(Arc::new(SqliteStore::open(path)?)))
}

/// Converts project records to stable public JSON.
///
/// - `projects`: Project records to serialize.
///
/// Public JSON values.
/// # Returns
/// The operation result.
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

/// Converts a scan file to stable public JSON.
///
/// - `file`: Scan file to serialize.
///
/// Public JSON file metadata.
/// # Returns
/// The operation result.
fn file_json(file: &ScanFile) -> serde_json::Value {
    serde_json::json!({
        "path": file.relative_path,
        "bytes": file.bytes.len(),
        "digest": blake3::hash(&file.bytes).to_hex().to_string(),
        "adapter": file.adapter,
        "status": file.status,
    })
}

/// Converts a project status to its stable public spelling.
///
/// - `status`: Project status to serialize.
///
/// Stable public status text.
/// # Returns
/// The operation result.
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
