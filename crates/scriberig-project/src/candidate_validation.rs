//! Validates durable project candidates in isolated source workspaces.

use super::{ProjectCandidate, ProjectError, ProjectService, candidate_support};
use std::sync::atomic::{AtomicBool, Ordering};

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
        self.validate_candidate_cancellable(id, &AtomicBool::new(false))
    }

    /// Validates a durable candidate while observing branch cancellation.
    ///
    /// Cancellation terminates the validator and leaves candidate validation pending,
    /// so a cancelled operation cannot authorize Apply.
    ///
    /// # Parameters
    /// - `id`: Stable candidate identity.
    /// - `cancelled`: Shared operation cancellation flag.
    ///
    /// # Returns
    /// Candidate updated with exact-state validation evidence.
    ///
    /// # Errors
    /// Returns an error when the candidate is missing, stale, cancelled, or unavailable.
    pub fn validate_candidate_cancellable(
        &self,
        id: &str,
        cancelled: &AtomicBool,
    ) -> Result<ProjectCandidate, ProjectError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }
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
        candidate.validation =
            candidate_support::validate_effects(&files, &candidate.effects, cancelled)?;
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }
}

/// Creates the stable cancellation error used by candidate validators.
///
/// # Returns
/// Structured cancellation outcome that leaves validation evidence unchanged.
fn cancelled_error() -> ProjectError {
    ProjectError {
        code: "operation_cancelled",
        message: "candidate validation was cancelled".to_owned(),
        diagnostics: Vec::new(),
    }
}
