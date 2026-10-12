//! Implements HTTP project catalog and onboarding routes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use scriberig_project::{ProjectService, ScanOptions};
use scriberig_store_sqlite::{ProjectRecord, ProjectStatus};
use serde_json::{Value, json};

use crate::http::HttpState;
use crate::server::ApplicationService;

/// Dispatches a typed project protocol request onto the shared project service.
///
/// # Parameters
/// - `state`: HTTP application state.
/// - `request`: Typed project operation and payload.
/// - `correlation_id`: Protocol correlation identity.
///
/// # Returns
/// A correlated typed project response.
pub(super) fn protocol<S: ApplicationService>(
    state: &HttpState<S>,
    request: &scriberig_protocol::ProjectRequest,
    correlation_id: String,
) -> scriberig_protocol::ResponseEnvelope {
    let body = serde_json::to_string(&request.payload).unwrap_or_default();
    let result = match request.operation.as_str() {
        "list" => list(state),
        "scan" => scan(state, &body),
        "import" => import(state, &body),
        "relink" => relink(state, &body),
        _ => Err((404, json!({"code": "unsupported_project_operation"}))),
    };
    let response = match result {
        Ok(value) => scriberig_protocol::Response::Project(scriberig_protocol::ProjectResponse {
            result: value,
        }),
        Err((_, value)) => scriberig_protocol::Response::Error(scriberig_protocol::ProtocolError {
            code: scriberig_protocol::ProtocolErrorCode::InvalidRequest,
            message: value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("project operation failed")
                .to_owned(),
            details: Some(value),
            correlation_id: Some(correlation_id.clone()),
        }),
    };
    scriberig_protocol::ResponseEnvelope {
        version: scriberig_protocol::ProtocolVersion::default(),
        correlation_id,
        response,
    }
}

/// Lists projects in the configured database.
pub(super) fn list<S: ApplicationService>(state: &HttpState<S>) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let projects = store.list_projects().map_err(storage_error)?;
    Ok(json!({
        "projects": projects.iter().map(project_json).collect::<Vec<_>>()
    }))
}

/// Registers and persists a deterministic project scan.
///
/// # Parameters
/// - `state`: HTTP application state.
/// - `body`: JSON object containing `path` and optional `name`.
///
/// # Returns
/// The project and persisted file manifest.
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
    let path = authorized_path(&state.allowed_project_roots, &input.path)?;
    let report = ProjectService::new(Arc::clone(store))
        .scan_with_options(
            input.name,
            path,
            ScanOptions {
                git_tracked: input.git_tracked,
            },
        )
        .map_err(|error| project_error(&error))?;
    let files = report
        .files
        .iter()
        .map(|file| {
            json!({
                "path": file.relative_path,
                "bytes": file.bytes.len(),
                "digest": blake3::hash(&file.bytes).to_hex().to_string(),
                "adapter": file.adapter,
                "status": file.status,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "project": project_json(&report.project),
        "files": files,
        "gitTracked": report.git_tracked,
    }))
}

/// Relinks a project to a new path while preserving its identity.
pub(super) fn relink<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: RelinkInput = serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let path = authorized_path(&state.allowed_project_roots, &input.path)?;
    let id = input.project.parse().map_err(|_| invalid_project())?;
    store.relink_project(id, path).map_err(storage_error)?;
    let project = store
        .project(id)
        .map_err(storage_error)?
        .ok_or_else(invalid_project)?;
    Ok(json!({"project": project_json(&project)}))
}

/// Imports the persisted scan into an immutable project snapshot.
///
/// # Parameters
/// - `state`: HTTP application state.
/// - `body`: JSON object containing the project identity.
///
/// # Returns
/// The imported file/entity counts and promoted snapshot.
///
/// # Errors
/// Returns an HTTP error tuple for malformed input, validation, or promotion
/// failures.
pub(super) fn import<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ImportInput = serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let id = input.project.parse().map_err(|_| invalid_project())?;
    let report = ProjectService::new(Arc::clone(store))
        .import(id)
        .map_err(|error| project_error(&error))?;
    Ok(json!({
        "project": project_json(&report.project),
        "files": report.files,
        "entities": report.entities,
        "snapshot": report.snapshot.to_string(),
    }))
}

/// Request body for a project scan.
#[derive(serde::Deserialize)]
struct ScanInput {
    /// Filesystem path to scan.
    path: String,
    /// Optional display name.
    name: Option<String>,
    /// Restrict the manifest to existing Git-indexed files.
    #[serde(default, alias = "gitTracked")]
    git_tracked: bool,
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

/// Converts a project-service failure into an HTTP response.
///
/// # Parameters
/// - `error`: Project-service failure.
///
/// # Returns
/// HTTP status and structured error body.
fn project_error(error: &scriberig_project::ProjectError) -> (u16, Value) {
    let status = if error.code == "project_not_found" {
        404
    } else {
        400
    };
    (
        status,
        json!({"code": error.code, "message": error.message}),
    )
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

/// Resolves a requested project path under one of the configured roots.
///
/// # Parameters
/// - `roots`: Canonical filesystem roots allowed by the server operator.
/// - `requested`: User-supplied project path.
///
/// # Returns
/// A canonical path contained by an allowed root.
///
/// # Errors
/// Returns a forbidden response when the path is missing or outside policy.
fn authorized_path(roots: &[PathBuf], requested: &str) -> Result<PathBuf, (u16, Value)> {
    let path = std::fs::canonicalize(Path::new(requested)).map_err(|error| {
        (
            400,
            json!({"code": "invalid_project_path", "message": error.to_string()}),
        )
    })?;
    if roots.iter().any(|root| path.starts_with(root)) {
        Ok(path)
    } else {
        Err((403, json!({"code": "project_path_forbidden"})))
    }
}

#[cfg(test)]
mod tests {
    use super::authorized_path;
    /// Rejects a project path outside the operator-selected root.
    #[test]
    fn path_policy_rejects_outside_root() {
        let root = std::env::temp_dir();
        let outside = std::path::Path::new("C:\\");
        let result = authorized_path(&[root], outside.to_string_lossy().as_ref());
        assert!(result.is_err());
    }

    /// Accepts a project path contained by a configured root.
    #[test]
    fn path_policy_accepts_existing_child() {
        let root = std::fs::canonicalize(std::env::temp_dir()).expect("temporary root");
        let result = authorized_path(std::slice::from_ref(&root), &root.to_string_lossy());
        assert!(result.is_ok());
    }
}
