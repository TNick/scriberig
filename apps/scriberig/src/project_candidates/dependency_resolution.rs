//! Reports bounded dependency acquisition evidence for project candidates.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_project::CandidateValidation;

use super::{candidate_id, validation_name};
use crate::agent_tool::LocalAgentTool;

impl<B: scriberig_app::ApplicationBackend> LocalAgentTool<'_, B> {
    /// Acquires dependencies fixed by a candidate lockfile in isolation.
    ///
    /// - `arguments`: Stable candidate identity.
    /// - `cancelled`: Shared branch cancellation flag.
    ///
    /// Durable dependency acquisition evidence bound to the candidate.
    ///
    /// Returns invalid input, stale-base, cancellation, or workspace failure.
    /// Acquisition failures are returned as durable failed evidence.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(crate) fn project_resolve_dependencies(
        &self,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        let id = candidate_id(arguments)?;
        let candidate = self
            .project_service()?
            .resolve_candidate_dependencies_cancellable(id, cancelled)
            .map_err(|error| Self::project_error(&error))?;
        let (summary, resolution) = match &candidate.validation {
            CandidateValidation::Pending => (
                format!(
                    "locked dependency acquisition remains pending for candidate {}",
                    candidate.id
                ),
                "pending",
            ),
            CandidateValidation::Passed => (
                format!(
                    "locked dependency acquisition passed for candidate {}",
                    candidate.id
                ),
                "passed",
            ),
            CandidateValidation::Failed { diagnostics } => (
                format!(
                    "locked dependency acquisition failed for candidate {}: {}",
                    candidate.id,
                    diagnostics
                        .iter()
                        .take(8)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                "failed",
            ),
        };
        let mut result = Self::result(
            summary,
            vec![
                format!("candidate:{}", candidate.id),
                format!("snapshot:{}", candidate.base_snapshot),
                format!("dependency_resolution:{resolution}"),
                format!("validation:{}", validation_name(&candidate.validation)),
                "validation_profile:cargo-dependency-resolution".to_owned(),
            ],
        );
        result.snapshot = Some(candidate.base_snapshot);
        result.object_type = Some("candidate_dependency_resolution".to_owned());
        Ok(result)
    }
}
