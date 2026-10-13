//! Implements trusted local project bootstrap and manifest review tools.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_app::{ApplicationBackend, ApplicationContext};
use scriberig_project::ScanOptions;

use crate::agent_tool::LocalAgentTool;
use crate::agent_tool_helpers::invalid_argument;

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Scans the trusted local working directory into a persisted project manifest.
    ///
    /// # Parameters
    /// - `super`: Parent agent tool module.
    /// - `context`: Trusted local bootstrap environment.
    /// - `arguments`: Optional project name and Git selection policy.
    ///
    /// # Returns
    /// Stable project and exact manifest identities with bounded counts.
    ///
    /// # Errors
    /// Returns an authorization, bootstrap-context, or scan failure.
    pub(super) fn project_scan(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let bootstrap = context
            .bootstrap
            .as_ref()
            .ok_or_else(|| ToolError::Application {
                category: "bootstrap_unavailable".to_owned(),
                message: "trusted local bootstrap directory is not available".to_owned(),
            })?;
        if arguments
            .get("path_ref")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value != "current_working_directory")
        {
            return Err(invalid_argument(
                "path_ref must be current_working_directory",
            ));
        }
        let name = arguments
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let git_tracked = arguments
            .get("git_tracked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let service = self.project_service()?;
        let report = service
            .scan_with_options(
                name,
                &bootstrap.working_directory,
                ScanOptions { git_tracked },
            )
            .map_err(|error| Self::project_error(&error))?;
        let manifest = service
            .manifest_summary(report.project.id)
            .map_err(|error| Self::project_error(&error))?;
        let mut result = Self::result(
            format!(
                "scan persisted for {} at {}: {} files ({} ready); review manifest {} before import",
                manifest.name,
                manifest.path,
                manifest.file_count,
                manifest.ready_count,
                manifest.manifest_id
            ),
            vec![report.project.id.to_string(), manifest.manifest_id],
        );
        result.object_type = Some("project_scan".to_owned());
        Ok(result)
    }

    /// Reviews a bounded page of the exact persisted scan manifest.
    ///
    /// # Parameters
    /// - `super`: Parent agent tool module.
    /// - `arguments`: Project, manifest digest, and optional page offset/limit.
    ///
    /// # Returns
    /// Manifest summary plus bounded path, digest, adapter, and status entries.
    ///
    /// # Errors
    /// Returns invalid arguments, stale manifest, or project lookup failures.
    pub(super) fn project_manifest(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let project = Self::project_id(arguments)?;
        let manifest_id = Self::manifest_id(arguments)?;
        let offset = usize::try_from(
            arguments
                .get("offset")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        )
        .unwrap_or(usize::MAX);
        let limit = usize::try_from(
            arguments
                .get("limit")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(50),
        )
        .unwrap_or(100);
        let service = self.project_service()?;
        let summary = service
            .manifest_summary(project)
            .map_err(|error| Self::project_error(&error))?;
        let page = service
            .manifest_page(project, &manifest_id, offset, limit)
            .map_err(|error| Self::project_error(&error))?;
        let mut handles = page
            .entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "path": entry.path,
                    "source_digest": entry.source_digest,
                    "adapter": entry.adapter,
                    "status": entry.status,
                })
                .to_string()
            })
            .collect::<Vec<_>>();
        if let Some(next) = page.next_offset {
            handles.push(format!("next_offset:{next}"));
        }
        Ok(Self::result(
            format!(
                "manifest {} for {} at {} has {} files ({} ready, {} other); page starts at {}",
                summary.manifest_id,
                summary.name,
                summary.path,
                summary.file_count,
                summary.ready_count,
                summary.other_count,
                offset
            ),
            handles,
        ))
    }

    /// Resolves the authoritative snapshot and semantic root after import.
    ///
    /// # Parameters
    /// - `super`: Parent agent tool module.
    /// - `arguments`: Provider arguments containing a project identity.
    ///
    /// # Returns
    /// Exact project, snapshot, and semantic root references.
    ///
    /// # Errors
    /// Returns invalid arguments or a project workspace failure.
    pub(super) fn project_workspace(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let project_id = Self::project_id(arguments)?;
        let workspace = self
            .project_service()?
            .workspace(project_id)
            .map_err(|error| Self::project_error(&error))?;
        let mut result = Self::result(
            format!(
                "active semantic workspace {} at snapshot {} has root {}",
                workspace.project.name, workspace.snapshot, workspace.root_entity
            ),
            vec![
                workspace.project.id.to_string(),
                workspace.snapshot.to_string(),
                workspace.root_entity.to_string(),
            ],
        );
        result.object_type = Some("project_workspace".to_owned());
        Ok(result)
    }
}
