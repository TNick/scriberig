//! Exposes explicit dependency-candidate integration to worker branches.

use scriberig_agent_core::{ToolError, ToolResult};

use super::LocalAgentTool;
use crate::agent_tool_helpers::invalid_argument;

impl<B: scriberig_app::ApplicationBackend> LocalAgentTool<'_, B> {
    /// Merges exact-base candidates received from fulfilled plan dependencies.
    ///
    /// - `arguments`: Left and right candidate handles from dependency results.
    ///
    /// New reviewable candidate handle without applying it to the project.
    ///
    /// Returns a structured compatibility or same-target conflict failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the requested candidates cannot be merged.
    pub(super) fn project_merge_candidates(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let left = candidate_argument(arguments, "left_candidate")?;
        let right = candidate_argument(arguments, "right_candidate")?;
        let merged = self
            .project_service()?
            .merge_project_candidates(left, right)
            .map_err(|error| Self::project_error(&error))?;
        let mut result = Self::result(
            format!(
                "integrated candidate {} combines {} and {} at exact base {}",
                merged.id, left, right, merged.base_snapshot
            ),
            vec![
                format!("candidate:{}", merged.id),
                format!("candidate_parent:{left}"),
                format!("candidate_parent:{right}"),
                format!("project:{}", merged.project),
                format!("snapshot:{}", merged.base_snapshot),
            ],
        );
        result.snapshot = Some(merged.base_snapshot);
        result.object_type = Some("project_candidate_merge".to_owned());
        Ok(result)
    }
}

/// Reads one non-empty candidate identity from provider arguments.
///
/// - `arguments`: Provider-supplied merge request.
/// - `name`: Required candidate field name.
///
/// Candidate identity text.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an invalid-argument error when the field is missing or empty.
fn candidate_argument<'a>(
    arguments: &'a serde_json::Value,
    name: &str,
) -> Result<&'a str, ToolError> {
    arguments
        .get(name)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_argument(name))
}
