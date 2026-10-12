//! Implements HTTP project catalog and filesystem scan endpoints.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::http::HttpState;
use crate::server::ApplicationService;
use scriberig_core::{EncodingVersion, EntityId, ObjectType, RootName};
use scriberig_store::{Revision, RootStore, Snapshot, SnapshotChange};
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

/// Imports the current filesystem contents into the project's immutable snapshot.
///
/// # Parameters
/// - `state`: HTTP application state.
/// - `body`: JSON object containing the project identity.
///
/// # Returns
/// The imported file count and promoted snapshot.
///
/// # Errors
/// Returns an HTTP error tuple for malformed input, storage, or filesystem
/// failures.
pub(super) fn import<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ImportInput = serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let id = input.project.parse().map_err(|_| invalid_project())?;
    let project = store
        .project(id)
        .map_err(storage_error)?
        .ok_or_else(invalid_project)?;
    let files = scan_source_files(&project.path).map_err(|error| filesystem_error(&error))?;
    let base = project
        .current_snapshot
        .unwrap_or_else(|| Snapshot::empty().id());
    let root = RootName::new(format!("project_{}", project.id)).map_err(storage_error)?;
    let expected = store.read_root(&root);
    let mut revisions = Vec::new();
    let mut changes = Vec::new();
    for file in files.iter().filter(|file| file.adapter != "unsupported") {
        let entity = source_entity(project.id, &file.path);
        let object_type =
            ObjectType::new(format!("project.file.{}", file.adapter)).map_err(storage_error)?;
        let revision = Revision::new(EncodingVersion::V1, object_type, file.bytes.clone())
            .map_err(|error| storage_error(format!("revision error: {error:?}")))?;
        changes.push(SnapshotChange::Set(entity, revision.id()));
        revisions.push(revision);
    }
    let snapshot = store
        .apply_revisions_and_promote(root, expected, base, &revisions, &changes)
        .map_err(|error| storage_error(format!("promotion error: {error:?}")))?;
    store
        .set_project_status(id, ProjectStatus::Imported, Some(snapshot))
        .map_err(storage_error)?;
    let project = store
        .project(id)
        .map_err(storage_error)?
        .ok_or_else(invalid_project)?;
    Ok(json!({"project": project_json(&project), "files": changes.len()}))
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

/// Request body for a project import.
#[derive(serde::Deserialize)]
struct ImportInput {
    /// Stable project identity.
    project: String,
}

/// Describes source bytes selected for project import.
struct SourceFile {
    /// Project-relative path.
    path: String,
    /// Exact source bytes.
    bytes: Vec<u8>,
    /// Selected source adapter.
    adapter: &'static str,
}

/// Scans source files while retaining bytes for import.
///
/// # Parameters
/// - `root`: Project directory to scan.
///
/// # Returns
/// Deterministically ordered source files.
///
/// # Errors
/// Returns a filesystem traversal or read error.
fn scan_source_files(root: &Path) -> std::io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    visit_source_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

/// Visits one source directory recursively.
///
/// # Parameters
/// - `root`: Project root used for relative paths.
/// - `directory`: Directory currently being visited.
/// - `files`: Source files being collected.
///
/// # Returns
/// Unit after the directory has been visited.
///
/// # Errors
/// Returns a filesystem traversal or read error.
fn visit_source_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<SourceFile>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if relative.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(
                    ".git"
                        | "target"
                        | "node_modules"
                        | "dist"
                        | "build"
                        | ".pages"
                        | "__pycache__"
                        | ".venv"
                        | ".cache"
                )
            )
        }) {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            visit_source_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(SourceFile {
                path: relative.to_string_lossy().replace('\\', "/"),
                bytes: std::fs::read(&path)?,
                adapter: adapter_for(&path),
            });
        }
    }
    Ok(())
}

/// Selects a source adapter by extension.
///
/// # Parameters
/// - `path`: File path to classify.
///
/// # Returns
/// The selected adapter name.
fn adapter_for(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("md" | "markdown") => "markdown",
        Some("rs") => "rust",
        Some("pdf") => "pdf",
        _ => "unsupported",
    }
}

/// Derives a stable entity identity for one project source file.
///
/// # Parameters
/// - `project`: Stable project identity.
/// - `path`: Project-relative file path.
///
/// # Returns
/// Stable entity identity for the source artifact.
fn source_entity(project: scriberig_core::ProjectId, path: &str) -> EntityId {
    let mut input = Vec::with_capacity(16 + path.len());
    input.extend_from_slice(project.as_bytes());
    input.extend_from_slice(path.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&blake3::hash(&input).as_bytes()[..16]);
    EntityId::from_bytes(bytes)
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
