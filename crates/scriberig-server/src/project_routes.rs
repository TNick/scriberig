//! Implements HTTP project catalog and filesystem scan endpoints.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::http::HttpState;
use crate::server::ApplicationService;
use scriberig_store_sqlite::{ProjectRecord, ProjectStatus};

/// Lists projects in the configured database.
pub(super) fn list<S: ApplicationService>(state: &HttpState<S>) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let projects = store.list_projects().map_err(storage_error)?;
    Ok(json!({"projects": projects.iter().map(project_json).collect::<Vec<_>>() }))
}

/// Registers and scans a project path.
///
/// # Parameters
/// - `state`: HTTP application state.
/// - `body`: JSON object containing `path` and optional `name`.
///
/// # Returns
/// The project and deterministic file manifest.
///
/// # Errors
/// Returns an HTTP error tuple for malformed input, storage, or filesystem
/// failures.
pub(super) fn scan<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ScanInput = serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let path = PathBuf::from(input.path);
    let name = input.name.or_else(|| {
        path.file_name()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned)
    });
    let project = store
        .register_project(name.unwrap_or_else(|| "project".to_owned()), &path)
        .map_err(storage_error)?;
    let files = scan_directory(&project.path).map_err(|error| filesystem_error(&error))?;
    store
        .set_project_status(project.id, ProjectStatus::ScanReady, None)
        .map_err(storage_error)?;
    Ok(json!({"project": project_json(&project), "files": files}))
}

/// Relinks a project to a new path.
pub(super) fn relink<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: RelinkInput = serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let id = input.project.parse().map_err(|_| invalid_project())?;
    store
        .relink_project(id, input.path)
        .map_err(storage_error)?;
    let project = store
        .project(id)
        .map_err(storage_error)?
        .ok_or_else(invalid_project)?;
    Ok(json!({"project": project_json(&project)}))
}

/// Request body for a project scan.
#[derive(serde::Deserialize)]
struct ScanInput {
    /// Filesystem path to scan.
    path: String,
    /// Optional display name.
    name: Option<String>,
}

/// Request body for a project relink.
#[derive(serde::Deserialize)]
struct RelinkInput {
    /// Stable project identity.
    project: String,
    /// New filesystem path.
    path: String,
}

/// Scans a directory using the default onboarding exclusions.
///
/// # Parameters
/// - `root`: Project directory to scan.
///
/// # Returns
/// Deterministically ordered file manifest entries.
///
/// # Errors
/// Returns a filesystem traversal or read error.
fn scan_directory(root: &Path) -> std::io::Result<Vec<Value>> {
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });
    Ok(files)
}

/// Visits one directory recursively.
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
fn visit(root: &Path, directory: &Path, files: &mut Vec<Value>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if relative.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(".git" | "target" | "node_modules" | "dist" | "build")
            )
        }) {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            visit(root, &path, files)?;
        } else if metadata.is_file() {
            let bytes = std::fs::read(&path)?;
            let adapter = match path.extension().and_then(|value| value.to_str()) {
                Some("md" | "markdown") => "markdown",
                Some("rs") => "rust",
                Some("pdf") => "pdf",
                _ => "unsupported",
            };
            files.push(json!({
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

/// Serializes one project record.
///
/// # Parameters
/// - `project`: Project record to serialize.
///
/// # Returns
/// Public JSON project metadata.
fn project_json(project: &ProjectRecord) -> Value {
    json!({
        "id": project.id.to_string(),
        "name": project.name,
        "path": project.path,
        "status": match project.status {
            ProjectStatus::Registered => "registered",
            ProjectStatus::ScanReady => "scan_ready",
            ProjectStatus::Imported => "imported",
            ProjectStatus::Failed => "failed",
        },
        "snapshot": project.current_snapshot.map(|value| value.to_string()),
    })
}

/// Creates a standard unavailable-service response.
///
/// # Returns
/// HTTP status and structured error body.
fn unavailable() -> (u16, Value) {
    (503, json!({"code": "project_catalog_unavailable"}))
}

/// Creates a standard invalid-request response.
///
/// # Parameters
/// - `error`: JSON decoding failure.
///
/// # Returns
/// HTTP status and structured error body.
fn invalid_request(error: &serde_json::Error) -> (u16, Value) {
    (
        400,
        json!({"code": "invalid_request", "message": error.to_string()}),
    )
}

/// Creates a standard invalid-project response.
///
/// # Returns
/// HTTP status and structured error body.
fn invalid_project() -> (u16, Value) {
    (404, json!({"code": "project_not_found"}))
}

/// Converts a storage failure into an HTTP error.
///
/// # Parameters
/// - `error`: Storage failure.
///
/// # Returns
/// HTTP status and structured error body.
fn storage_error(error: impl std::fmt::Display) -> (u16, Value) {
    (
        500,
        json!({"code": "storage_error", "message": error.to_string()}),
    )
}

/// Converts a filesystem failure into an HTTP error.
///
/// # Parameters
/// - `error`: Filesystem failure.
///
/// # Returns
/// HTTP status and structured error body.
fn filesystem_error(error: &std::io::Error) -> (u16, Value) {
    (
        400,
        json!({"code": "filesystem_error", "message": error.to_string()}),
    )
}
