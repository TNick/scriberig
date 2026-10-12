//! Implements HTTP project catalog and onboarding routes.

use std::sync::Arc;

use scriberig_project::{CandidateValidation, ProjectService, ScanOptions};
use scriberig_store::SnapshotStore;
use serde_json::{Value, json};
#[path = "project_agent.rs"]
mod project_agent;
#[path = "project_routes_support.rs"]
mod project_routes_support;
#[path = "project_workflow.rs"]
mod project_workflow;
use project_routes_support::{
    authorized_path, invalid_project, invalid_request, project_error, project_json, storage_error,
    unavailable, workspace_json,
};

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
        "workspace" => workspace(state, &body),
        "symbols" => symbols(state, &body),
        "projection" => projection(state, &body),
        "history" => history(state, &body),
        "propose" => propose(state, &body),
        "apply" => apply(state, &body),
        "agent" => project_agent::agent(state, &body),
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

/// Dispatches the browser transport's direct project operation shape.
///
/// # Parameters
/// - `state`: HTTP application state.
/// - `body`: JSON project request containing `operation` and `payload`.
///
/// # Returns
/// The unwrapped project result expected by the browser client.
///
/// # Errors
/// Returns a structured HTTP error for malformed or failed project operations.
pub(super) fn direct<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let request: scriberig_protocol::ProjectRequest =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    match request.operation.as_str() {
        "list" => list(state),
        "scan" => scan(state, &request.payload.to_string()),
        "import" => import(state, &request.payload.to_string()),
        "workspace" => workspace(state, &request.payload.to_string()),
        "symbols" => symbols(state, &request.payload.to_string()),
        "projection" => projection(state, &request.payload.to_string()),
        "history" => history(state, &request.payload.to_string()),
        "propose" => propose(state, &request.payload.to_string()),
        "apply" => apply(state, &request.payload.to_string()),
        "agent" => project_agent::agent(state, &request.payload.to_string()),
        "relink" => relink(state, &request.payload.to_string()),
        _ => Err((
            404,
            json!({
                "code": "unsupported_project_operation",
                "message": "unsupported project operation"
            }),
        )),
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
    let workspace = ProjectService::new(Arc::clone(store))
        .workspace(id)
        .map_err(|error| project_error(&error))?;
    Ok(json!({
        "project": project_json(&report.project),
        "files": report.files,
        "entities": report.entities,
        "snapshot": report.snapshot.to_string(),
        "workspace": workspace_json(&workspace),
        "diagnostics": [],
    }))
}

/// Resolves one project's active semantic workspace.
pub(super) fn workspace<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ImportInput = serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let id = input.project.parse().map_err(|_| invalid_project())?;
    let value = ProjectService::new(Arc::clone(store))
        .workspace(id)
        .map_err(|error| project_error(&error))?;
    Ok(workspace_json(&value))
}

/// Lists named semantic symbols owned by one active project.
pub(super) fn symbols<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ImportInput = serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let id = input.project.parse().map_err(|_| invalid_project())?;
    let workspace = ProjectService::new(Arc::clone(store))
        .workspace(id)
        .map_err(|error| project_error(&error))?;
    let symbols = ProjectService::new(Arc::clone(store))
        .symbols(id)
        .map_err(|error| project_error(&error))?
        .into_iter()
        .map(|symbol| {
            json!({
                "entityId": symbol.entity.to_string(),
                "snapshotId": symbol.snapshot.to_string(),
                "path": symbol.path,
                "kind": symbol.kind,
                "name": symbol.name.unwrap_or_else(|| "source".to_owned()),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "workspace": workspace_json(&workspace), "symbols": symbols }))
}

/// Renders the exact source projection for one project entity.
pub(super) fn projection<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ProjectionInput =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let project = input.project.parse().map_err(|_| invalid_project())?;
    let entity = input.entity.parse().map_err(|_| invalid_project())?;
    let projection = ProjectService::new(Arc::clone(store))
        .projection(project, entity)
        .map_err(|error| project_error(&error))?;
    Ok(json!({
        "project": projection.project.to_string(),
        "snapshot": projection.snapshot.to_string(),
        "entityId": projection.entity.to_string(),
        "name": projection.name,
        "kind": projection.kind,
        "path": projection.path,
        "text": String::from_utf8_lossy(&projection.text),
        "editable": projection.editable,
        "stale": false,
    }))
}

/// Returns the immutable snapshot lineage for one project.
pub(super) fn history<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ProjectInput =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let project = input.project.parse().map_err(|_| invalid_project())?;
    let workspace = ProjectService::new(Arc::clone(store))
        .workspace(project)
        .map_err(|error| project_error(&error))?;
    let mut snapshots = vec![workspace.snapshot.to_string()];
    let mut current = workspace.snapshot;
    for _ in 0..16 {
        let parents = store
            .parents(current)
            .map_err(|error| storage_error(format!("{error:?}")))?;
        let Some(parent) = parents.first().copied() else {
            break;
        };
        snapshots.push(parent.to_string());
        current = parent;
    }
    Ok(json!({"project": project.to_string(), "snapshots": snapshots}))
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

/// Request body for project-scoped history.
#[derive(serde::Deserialize)]
struct ProjectInput {
    /// Stable project identity.
    project: String,
}

/// Request body for a project import.
#[derive(serde::Deserialize)]
struct ImportInput {
    /// Stable project identity.
    project: String,
}

/// Request body for workspace projection lookup.
#[derive(serde::Deserialize)]
struct ProjectionInput {
    /// Stable project identity.
    project: String,
    /// Stable semantic entity identity.
    entity: String,
}

/// Creates and validates a source candidate without changing canonical state.
pub(super) fn propose<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    project_workflow::propose(state, body)
}

/// Applies a validated candidate with snapshot and filesystem compare-and-swap.
pub(super) fn apply<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    project_workflow::apply(state, body)
}
