//! Implements local project catalog and filesystem scan commands.

use std::path::{Path, PathBuf};

use scriberig_store_sqlite::{ProjectRecord, ProjectStatus, SqliteStore};

use clap::{Args, Subcommand};

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
    let mut files = Vec::new();
    scan_directory(path, path, &mut files)?;
    files.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });
    Ok(files)
}

/// Recursively visits one directory while applying default exclusions.
///
/// # Parameters
/// - `root`: Project root used for relative paths.
/// - `directory`: Directory currently being visited.
/// - `files`: Manifest entries being collected.
///
/// # Returns
/// Unit after the directory has been visited.
///
/// # Errors
/// Returns a filesystem traversal or read error.
fn scan_directory(
    root: &Path,
    directory: &Path,
    files: &mut Vec<serde_json::Value>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if is_excluded(relative) {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            scan_directory(root, &path, files)?;
        } else if metadata.is_file() {
            let bytes = std::fs::read(&path)?;
            let adapter = adapter_for(&path);
            files.push(serde_json::json!({
                "path": relative.to_string_lossy().replace('\\', "/"),
                "bytes": bytes.len(),
                "digest": blake3::hash(&bytes).to_hex().to_string(),
                "adapter": adapter,
                "status": if adapter == "unsupported" { "unsupported" } else { "ready" },
            }));
        }
    }
    Ok(())
}

/// Returns whether a relative path belongs to a default-excluded directory.
///
/// # Parameters
/// - `path`: Relative path to classify.
///
/// # Returns
/// Whether the path should be skipped.
fn is_excluded(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".git" | "target" | "node_modules" | "dist" | "build")
        )
    })
}

/// Selects the first supported adapter by file extension.
///
/// # Parameters
/// - `path`: File path to classify.
///
/// # Returns
/// The adapter name or `unsupported`.
fn adapter_for(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("md" | "markdown") => "markdown",
        Some("rs") => "rust",
        Some("pdf") => "pdf",
        _ => "unsupported",
    }
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
