//! Provides shared serialization, error, and path-policy helpers for project routes.

use std::path::{Path, PathBuf};

use scriberig_store_sqlite::{ProjectRecord, ProjectStatus};
use serde_json::{Value, json};

/// Serializes a resolved project workspace.
///
/// - `workspace`: Workspace to serialize.
/// - `super`: Parent route module visibility marker.
///
/// Public workspace JSON.
/// # Returns
/// The operation result.
pub(super) fn workspace_json(workspace: &scriberig_project::ProjectWorkspace) -> Value {
    json!({
        "project": project_json(&workspace.project),
        "snapshot": workspace.snapshot.to_string(),
        "rootEntity": workspace.root_entity.to_string(),
    })
}

/// Serializes one project record.
///
/// - `project`: Project record to serialize.
/// - `super`: Parent route module visibility marker.
///
/// Public project JSON.
/// # Returns
/// The operation result.
pub(super) fn project_json(project: &ProjectRecord) -> Value {
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
/// - `super`: Parent route module visibility marker.
///
/// HTTP status and structured error body.
/// # Returns
/// The operation result.
pub(super) fn unavailable() -> (u16, Value) {
    (503, json!({"code": "project_catalog_unavailable"}))
}

/// Creates a standard invalid-request response.
///
/// - `error`: JSON decoding error.
/// - `super`: Parent route module visibility marker.
///
/// HTTP status and structured error body.
/// # Returns
/// The operation result.
pub(super) fn invalid_request(error: &serde_json::Error) -> (u16, Value) {
    (
        400,
        json!({"code": "invalid_request", "message": error.to_string()}),
    )
}

/// Creates a standard invalid-project response.
///
/// - `super`: Parent route module visibility marker.
///
/// HTTP status and structured error body.
/// # Returns
/// The operation result.
pub(super) fn invalid_project() -> (u16, Value) {
    (404, json!({"code": "project_not_found"}))
}

/// Converts a project-service failure into an HTTP response.
///
/// - `error`: Project-service failure.
/// - `super`: Parent route module visibility marker.
///
/// HTTP status and structured error body.
/// # Returns
/// The operation result.
pub(super) fn project_error(error: &scriberig_project::ProjectError) -> (u16, Value) {
    let status = if error.code == "project_not_found" {
        404
    } else {
        400
    };
    (
        status,
        json!({
            "code": error.code,
            "message": error.message,
            "diagnostics": error.diagnostics.iter().map(|diagnostic| json!({
                "entity": diagnostic.entity.map(|entity| entity.to_string()),
                "path": diagnostic.path,
                "adapter": diagnostic.adapter,
                "stage": diagnostic.stage,
                "code": diagnostic.code,
                "message": diagnostic.message,
                "retryable": diagnostic.retryable,
            })).collect::<Vec<_>>(),
        }),
    )
}

/// Converts a storage failure into an HTTP error.
///
/// - `error`: Storage failure.
/// - `super`: Parent route module visibility marker.
///
/// HTTP status and structured error body.
/// # Returns
/// The operation result.
pub(super) fn storage_error(error: impl std::fmt::Display) -> (u16, Value) {
    (
        500,
        json!({"code": "storage_error", "message": error.to_string()}),
    )
}

/// Resolves a requested project path under configured roots.
///
/// - `roots`: Authorized filesystem roots.
/// - `requested`: User-supplied project path.
/// - `super`: Parent route module visibility marker.
///
/// A canonical path contained by an allowed root.
///
/// Returns a forbidden response when the path is missing or outside policy.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn authorized_path(roots: &[PathBuf], requested: &str) -> Result<PathBuf, (u16, Value)> {
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
    use super::{authorized_path, project_error};

    /// Preserves an exact semantic entity target in a project error response.
    #[test]
    fn project_error_serializes_target_entity_diagnostic() {
        let entity = scriberig_core::EntityId::new();
        let error = scriberig_project::ProjectError {
            code: "dxf_invalid_coordinate",
            message: "line coordinates must be finite numeric values".to_owned(),
            diagnostics: vec![scriberig_project::ImportDiagnostic {
                entity: Some(entity),
                path: "roads.dxf".to_owned(),
                adapter: "dxf".to_owned(),
                stage: "edit".to_owned(),
                code: "dxf_invalid_coordinate".to_owned(),
                message: "line coordinates must be finite numeric values".to_owned(),
                retryable: false,
            }],
        };

        let (status, body) = project_error(&error);

        assert_eq!(status, 400);
        assert_eq!(body["diagnostics"][0]["entity"], entity.to_string());
        assert_eq!(body["diagnostics"][0]["path"], "roads.dxf");
    }

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
