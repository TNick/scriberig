//! Applies validated aggregate project candidates with exact-state preflight.

use std::path::PathBuf;

use scriberig_core::{ProjectId, SnapshotId};
use scriberig_store_sqlite::{ProjectEffectPathRecord, ProjectEffectRecord};
use sr_projection_fs::{FileSystem, LocalFileSystem};

#[path = "candidate_apply_effects.rs"]
mod effects;
use effects::{materialize_effects, preflight_effects, preflight_project_tree};

use super::{
    CandidateApplyReport, CandidateValidation, CandidateValidationProfile, ProjectCandidate,
    ProjectError, ProjectService, ProjectSourceEffect,
};

impl ProjectService {
    /// Applies a validated candidate after aggregate exact-base preflight.
    ///
    /// - `candidate`: Durable candidate returned by shared project operations.
    ///
    /// The successor snapshot and durable per-path materialization receipts.
    ///
    /// Returns stale, validation, ownership, path, conflict, I/O, or import errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn apply_candidate(
        &self,
        candidate: &ProjectCandidate,
    ) -> Result<CandidateApplyReport, ProjectError> {
        self.apply_candidate_with_filesystem(candidate, &LocalFileSystem)
    }

    /// Applies one candidate using the supplied filesystem boundary.
    ///
    /// This internal boundary keeps live effects on the shared filesystem
    /// contract and permits deterministic recovery testing without global flags.
    ///
    /// - `candidate`: Durable candidate returned by shared project operations.
    /// - `filesystem`: Filesystem capability used only for candidate effects.
    ///
    /// The successor snapshot and durable per-path materialization receipts.
    ///
    /// Returns stale, validation, ownership, path, conflict, I/O, or import errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn apply_candidate_with_filesystem(
        &self,
        candidate: &ProjectCandidate,
        filesystem: &dyn FileSystem,
    ) -> Result<CandidateApplyReport, ProjectError> {
        let durable = self
            .candidate(&candidate.id)?
            .ok_or_else(|| candidate_error("candidate_not_found", &candidate.id))?;
        if !Self::same_candidate_content(&durable, candidate) {
            return Err(candidate_error(
                "candidate_mismatch",
                "apply request differs from the durable candidate",
            ));
        }
        require_validation(&durable)?;
        self.record_effect(&durable, None, "attempted", None)?;
        let workspace = self.workspace(durable.project)?;
        if workspace.snapshot != durable.base_snapshot {
            let error = candidate_error(
                "stale_candidate",
                "project snapshot advanced since candidate creation",
            );
            self.record_effect(&durable, None, "failed", Some(&error.message))?;
            return Err(error);
        }
        let base_files =
            super::base::project_files_at(&self.store, durable.project, durable.base_snapshot)?;
        let root = self.project_path(durable.project)?;
        preflight_project_tree(&base_files, &durable.effects, &root)?;
        let plans = preflight_effects(self, &durable, &base_files, &root)?;
        if let Err(error) =
            materialize_effects(self, &durable, &plans, &base_files, &root, filesystem)
        {
            self.record_effect(&durable, None, "unknown", Some(&error.message))?;
            return Err(error);
        }
        let refresh =
            super::apply_scope::candidate_scan_paths(&root, &base_files, &durable.effects)
                .and_then(|paths| match paths {
                    Some(paths) => {
                        self.scan_with_paths(Some(workspace.project.name.clone()), &root, &paths)
                    }
                    None => self.scan(Some(workspace.project.name.clone()), &root),
                });
        if let Err(error) = refresh {
            self.record_effect(&durable, None, "unknown", Some(&error.message))?;
            return Err(error);
        }
        let imported = match self.import(durable.project) {
            Ok(imported) => imported,
            Err(error) => {
                self.record_effect(&durable, None, "unknown", Some(&error.message))?;
                return Err(error);
            }
        };
        self.record_effect(&durable, Some(imported.snapshot), "applied", None)?;
        super::candidate_support::cleanup_candidate_target(&durable.id);
        Ok(CandidateApplyReport {
            candidate: durable.id.clone(),
            base_snapshot: durable.base_snapshot,
            successor_snapshot: imported.snapshot,
            path: durable.path.clone(),
            paths: durable
                .effects
                .iter()
                .map(|effect| effect.path.clone())
                .collect(),
            effect_state: "applied".to_owned(),
        })
    }

    /// Compares immutable candidate identity and source meaning, excluding validation output.
    ///
    /// - `durable`: Candidate loaded from shared storage.
    /// - `requested`: Candidate supplied to the apply operation.
    ///
    /// Whether project/base/effect identity and exact source bytes match.
    /// # Returns
    /// The operation result.
    fn same_candidate_content(durable: &ProjectCandidate, requested: &ProjectCandidate) -> bool {
        durable.id == requested.id
            && durable.project == requested.project
            && durable.base_snapshot == requested.base_snapshot
            && durable.entity == requested.entity
            && durable.path == requested.path
            && durable.old == requested.old
            && durable.new == requested.new
            && durable.effects == requested.effects
    }

    /// Records aggregate filesystem effect status.
    ///
    /// - `candidate`: Candidate whose status is changing.
    /// - `successor`: Promoted snapshot when canonical import completed.
    /// - `state`: Aggregate materialization state.
    /// - `diagnostic`: Safe failure or recovery detail.
    ///
    /// Unit after the aggregate receipt is persisted.
    ///
    /// Returns a storage error when receipt persistence fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn record_effect(
        &self,
        candidate: &ProjectCandidate,
        successor: Option<SnapshotId>,
        state: &str,
        diagnostic: Option<&str>,
    ) -> Result<(), ProjectError> {
        self.store
            .put_project_effect(&ProjectEffectRecord {
                candidate: candidate.id.clone(),
                project: candidate.project,
                base_snapshot: candidate.base_snapshot,
                successor_snapshot: successor,
                path: candidate.path.clone(),
                state: state.to_owned(),
                diagnostic: diagnostic.map(ToOwned::to_owned),
            })
            .map_err(crate::errors::storage_error)
    }

    /// Records the durable state of one source effect.
    ///
    /// - `candidate`: Candidate owning the effect.
    /// - `effect`: Path effect whose state is recorded.
    /// - `state`: Per-path receipt state.
    /// - `observed`: Bytes observed after the effect, when present.
    /// - `diagnostic`: Safe failure detail.
    ///
    /// Unit after the path receipt is durable.
    ///
    /// Returns a storage error when the receipt cannot be persisted.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn record_effect_path(
        &self,
        candidate: &ProjectCandidate,
        effect: &ProjectSourceEffect,
        state: &str,
        observed: Option<&[u8]>,
        diagnostic: Option<&str>,
    ) -> Result<(), ProjectError> {
        self.store
            .put_project_effect_path(&ProjectEffectPathRecord {
                candidate: candidate.id.clone(),
                path: effect.path.clone(),
                state: state.to_owned(),
                expected_digest: effect.old.as_deref().map(digest),
                observed_digest: observed.map(digest),
                diagnostic: diagnostic.map(ToOwned::to_owned),
            })
            .map_err(crate::errors::storage_error)
    }

    /// Resolves the authorized project root from durable ownership data.
    ///
    /// - `project`: Project identity.
    /// - `super`: Parent candidate module visibility.
    ///
    /// Registered project root path.
    ///
    /// Returns an error when the project is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn project_path(&self, project: ProjectId) -> Result<PathBuf, ProjectError> {
        self.store
            .project(project)
            .map_err(crate::errors::storage_error)?
            .map(|record| record.path)
            .ok_or_else(|| candidate_error("project_not_found", &project.to_string()))
    }
}

/// Requires durable validation success before any filesystem effect.
///
/// - `candidate`: Candidate whose validation state is checked.
///
/// Unit when the candidate has passed validation.
///
/// Returns a validation-pending or validation-failed error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn require_validation(candidate: &ProjectCandidate) -> Result<(), ProjectError> {
    match &candidate.validation {
        CandidateValidation::Passed
            if candidate.validation_profile.as_deref()
                == Some(CandidateValidationProfile::CargoDependencyResolution.identifier()) =>
        {
            Err(candidate_error(
                "validation_pending",
                "dependency acquisition succeeded; candidate code validation is still required",
            ))
        }
        CandidateValidation::Passed => Ok(()),
        CandidateValidation::Pending => Err(candidate_error(
            "validation_pending",
            "candidate requires exact-state validation before application",
        )),
        CandidateValidation::Failed { diagnostics } => Err(candidate_error(
            "validation_failed",
            &diagnostics.join("; "),
        )),
    }
}

/// Creates a project candidate error with a bounded message.
///
/// - `code`: Stable error category.
/// - `message`: Safe error detail.
///
/// Structured project error.
/// # Returns
/// The operation result.
fn candidate_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.chars().take(4096).collect(),
        diagnostics: Vec::new(),
    }
}

/// Computes the BLAKE3 digest for exact bytes.
///
/// - `bytes`: Exact source bytes.
///
/// Fixed size digest.
/// # Returns
/// The operation result.
fn digest(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

#[cfg(test)]
#[path = "candidate_apply_tests.rs"]
mod tests;
