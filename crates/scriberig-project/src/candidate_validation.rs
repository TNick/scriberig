//! Validates durable project candidates in isolated source workspaces.

use super::{ProjectCandidate, ProjectError, ProjectService, candidate_support};

impl ProjectService {
    /// Validates a durable candidate reconstructed from exact base source bytes.
    ///
    /// # Parameters
    /// - `id`: Stable candidate identity.
    ///
    /// # Returns
    /// Candidate updated with validation evidence.
    ///
    /// # Errors
    /// Returns an error when the candidate is missing, stale, or cannot be loaded.
    pub fn validate_candidate(&self, id: &str) -> Result<ProjectCandidate, ProjectError> {
        let mut candidate = self.candidate(id)?.ok_or_else(|| ProjectError {
            code: "candidate_not_found",
            message: id.to_owned(),
            diagnostics: Vec::new(),
        })?;
        let workspace = self.workspace(candidate.project)?;
        if workspace.snapshot != candidate.base_snapshot {
            return Err(ProjectError {
                code: "stale_candidate",
                message: "project snapshot advanced since candidate creation".to_owned(),
                diagnostics: Vec::new(),
            });
        }
        let files =
            super::base::project_files_at(&self.store, candidate.project, candidate.base_snapshot)?;
        candidate.validation = candidate_support::validate_effects(&files, &candidate.effects);
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }
}
