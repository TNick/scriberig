//! Implements agent-assisted project import recovery operations.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_app::ApplicationBackend;
use scriberig_project::{ProjectService, ScanOptions};

use crate::agent_tool::LocalAgentTool;
use crate::agent_tool_helpers::invalid_argument;

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Imports a scanned project and returns structured recovery diagnostics.
    ///
    /// - `arguments`: Provider arguments containing a project identity.
    /// - `super`: Parent agent tool module visibility marker.
    ///
    /// Import summary or diagnostic handles for source repair.
    ///
    /// Returns invalid arguments or a structured import failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn project_import(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let project_id = Self::project_id(arguments)?;
        let manifest_id = Self::manifest_id(arguments)?;
        let project = self
            .project_service()?
            .import_manifest(project_id, &manifest_id);
        match project {
            Ok(report) => Ok(Self::result(
                format!(
                    "project import succeeded: {} file(s), {} entity(ies), snapshot {}",
                    report.files, report.entities, report.snapshot
                ),
                vec![report.snapshot.to_string()],
            )),
            Err(error) => Err(Self::project_error(&error)),
        }
    }

    /// Rescans a project after an authorized source repair.
    ///
    /// - `arguments`: Provider arguments containing project path and Git policy.
    /// - `super`: Parent agent tool module visibility marker.
    ///
    /// Scan summary and the stable project handle.
    ///
    /// Returns invalid arguments or a filesystem and Git selection failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn project_rescan(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let service = self.project_service()?;
        let path = arguments
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("path"))?;
        let name = arguments
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let git_tracked = arguments
            .get("git_tracked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let report = service
            .scan_with_options(name, path, ScanOptions { git_tracked })
            .map_err(|error| Self::project_error(&error))?;
        Ok(Self::result(
            format!(
                "project rescan succeeded: {} file(s), project {}",
                report.files.len(),
                report.project.id
            ),
            vec![report.project.id.to_string()],
        ))
    }

    /// Repairs one project-relative file before an explicit rescan and retry.
    ///
    /// - `arguments`: Provider arguments containing project, relative path, and content.
    /// - `super`: Parent agent tool module visibility marker.
    ///
    /// A stable repaired-file handle.
    ///
    /// Returns invalid arguments, path traversal, missing project, or filesystem failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn project_repair_file(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let store = self
            .project_store
            .as_ref()
            .ok_or_else(|| ToolError::Application {
                category: "project_recovery_unavailable".to_owned(),
                message: "project recovery is not configured for this runtime".to_owned(),
            })?;
        let project = Self::project_id(arguments)?;
        let relative = arguments
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("path"))?;
        let content = arguments
            .get("content")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("content"))?;
        let relative_path = std::path::Path::new(relative);
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(ToolError::Application {
                category: "invalid_request".to_owned(),
                message: "repair path must remain project-relative".to_owned(),
            });
        }
        let record = store
            .project(project)
            .map_err(|error| ToolError::Application {
                category: "storage_error".to_owned(),
                message: error.to_string(),
            })?
            .ok_or_else(|| ToolError::Application {
                category: "project_not_found".to_owned(),
                message: project.to_string(),
            })?;
        let path = record.path.join(relative_path);
        std::fs::write(&path, content).map_err(|error| ToolError::Application {
            category: "filesystem_error".to_owned(),
            message: error.to_string(),
        })?;
        Ok(Self::result(
            format!("repaired project file {relative}"),
            vec![relative.to_owned()],
        ))
    }

    /// Resolves the project service required by recovery operations.
    ///
    /// - `super`: Parent agent tool module.
    ///
    /// The configured project service.
    ///
    /// Returns a stable unavailable error when this runtime has no project store.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn project_service(&self) -> Result<&ProjectService, ToolError> {
        self.project.as_ref().ok_or_else(|| ToolError::Application {
            category: "project_recovery_unavailable".to_owned(),
            message: "project recovery is not configured for this runtime".to_owned(),
        })
    }

    /// Parses a project identity from provider arguments.
    ///
    /// - `super`: Parent agent tool module.
    /// - `arguments`: Provider argument object.
    ///
    /// Parsed project identity.
    ///
    /// Returns a structured invalid-request failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn project_id(
        arguments: &serde_json::Value,
    ) -> Result<scriberig_core::ProjectId, ToolError> {
        let value = arguments
            .get("project")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("project"))?;
        value
            .parse()
            .map_err(|error| invalid_argument(&format!("project {value} ({error})")))
    }

    /// Parses the exact manifest digest argument.
    ///
    /// - `super`: Parent agent tool module.
    /// - `arguments`: Provider argument object.
    ///
    /// Lowercase hexadecimal manifest identity.
    ///
    /// Returns invalid-request when the digest is missing or malformed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn manifest_id(arguments: &serde_json::Value) -> Result<String, ToolError> {
        let value = arguments
            .get("manifest_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("manifest_id"))?;
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(invalid_argument(
                "manifest_id (expected 64 hexadecimal characters)",
            ));
        }
        Ok(value.to_ascii_lowercase())
    }

    /// Converts a project failure into a bounded provider-facing result.
    ///
    /// - `super`: Parent agent tool module.
    /// - `error`: Structured project failure.
    ///
    /// Provider-neutral diagnostic failure.
    /// # Returns
    /// The operation result.
    pub(super) fn project_error(error: &scriberig_project::ProjectError) -> ToolError {
        let diagnostics = error
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}:{}:{}:{}",
                    diagnostic.path, diagnostic.adapter, diagnostic.code, diagnostic.message
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        ToolError::Application {
            category: error.code.to_owned(),
            message: format!("{} diagnostics={diagnostics}", error.message),
        }
    }
}
