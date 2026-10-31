//! Validates complete source-effect candidates in isolated workspaces.

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};

use scriberig_core::EntityId;
use scriberig_store_sqlite::ProjectFileRecord;

use crate::{CandidateValidation, CandidateValidationProfile, ProjectError};

use super::{
    ProcessFailure, candidate_target, docx_validation, dxf_validation, geopackage_validation,
    qgis_validation, run_candidate_cargo, safe_join, write_candidate_file, xlsx_validation,
};

#[allow(
    clippy::too_many_arguments,
    reason = "The validation request is a borrowed view over one candidate and its cancellation signal, kept explicit at this internal boundary."
)]
/// Validates a source-effect set in an isolated tree reconstructed from base bytes.
///
/// # Parameters
/// - `candidate_id`: Stable identity used to isolate the validation workspace.
/// - `files`: Exact project source manifest for the candidate base.
/// - `effects`: Ordered candidate source effects.
/// - `profile`: Validation commands and bounds selected for this project.
/// - `semantic_operation`: Exact semantic operation associated with the candidate.
/// - `semantic_target`: Exact semantic entity selected by the operation.
/// - `semantic_changes`: Typed before/after values recorded by the candidate.
/// - `cancelled`: Shared cancellation flag for the owning operation.
///
/// # Returns
/// Candidate validation state with bounded process diagnostics.
///
/// # Errors
/// Returns cancellation when validation is stopped by its owning operation.
pub(crate) fn validate_effects(
    candidate_id: &str,
    files: &[ProjectFileRecord],
    effects: &[super::super::ProjectSourceEffect],
    profile: CandidateValidationProfile,
    semantic_operation: Option<&str>,
    semantic_target: Option<EntityId>,
    semantic_changes: &[super::super::SemanticCandidateChange],
    cancelled: &AtomicBool,
) -> Result<CandidateValidation, ProjectError> {
    let _lock = match super::super::validation_lock::acquire(cancelled) {
        Ok(guard) => guard,
        Err(super::super::validation_lock::ValidationLockError::Cancelled) => {
            return Err(ProjectError {
                code: "operation_cancelled",
                message: "candidate validation was cancelled".to_owned(),
                diagnostics: Vec::new(),
            });
        }
        Err(super::super::validation_lock::ValidationLockError::Failed(error)) => {
            return Ok(CandidateValidation::Failed {
                diagnostics: vec![error],
            });
        }
    };
    let workspace_id = blake3::hash(candidate_id.as_bytes()).to_hex().to_string();
    let root = std::env::temp_dir().join(format!("scriberig-project-validation-{workspace_id}"));
    let target = candidate_target(candidate_id);
    let _ = fs::remove_dir_all(&root);
    let result = (|| {
        if cancelled.load(Ordering::Acquire) {
            return Err(ProcessFailure::Cancelled);
        }
        fs::create_dir_all(&root).map_err(|error| ProcessFailure::Failed(error.to_string()))?;
        for file in files {
            if cancelled.load(Ordering::Acquire) {
                return Err(ProcessFailure::Cancelled);
            }
            write_candidate_file(&root, &file.relative_path, &file.bytes)
                .map_err(ProcessFailure::Failed)?;
        }
        for effect in effects {
            if cancelled.load(Ordering::Acquire) {
                return Err(ProcessFailure::Cancelled);
            }
            let path = safe_join(&root, &effect.path)
                .map_err(|error| ProcessFailure::Failed(error.message))?;
            match &effect.new {
                Some(bytes) => write_candidate_file(&root, &effect.path, bytes)
                    .map_err(ProcessFailure::Failed)?,
                None => {
                    if path.exists() {
                        fs::remove_file(path)
                            .map_err(|error| ProcessFailure::Failed(error.to_string()))?;
                    }
                }
            }
        }
        if profile.is_xlsx_package() {
            xlsx_validation::validate_xlsx_packages(&root, effects, cancelled)
        } else if profile.is_docx_package() {
            docx_validation::validate_docx_packages(
                &root,
                files,
                effects,
                semantic_operation,
                semantic_target,
                cancelled,
            )
        } else if profile.is_dxf_package() {
            dxf_validation::validate_dxf_packages(
                &root,
                files,
                effects,
                semantic_operation,
                semantic_target,
                semantic_changes,
                cancelled,
            )
        } else if profile.is_geopackage_package() {
            geopackage_validation::validate_geopackage_packages(&root, effects, cancelled)
        } else if profile.is_qgis_package() {
            qgis_validation::validate_qgis_renames(&root, effects, semantic_operation, cancelled)
        } else {
            run_candidate_cargo(&root, &target, profile, cancelled)
        }
    })();
    let _ = fs::remove_dir_all(&root);
    match result {
        Ok(diagnostics) if diagnostics.is_empty() => Ok(CandidateValidation::Passed),
        Ok(diagnostics) => {
            let _ = fs::remove_dir_all(&target);
            Ok(CandidateValidation::Failed { diagnostics })
        }
        Err(ProcessFailure::Cancelled) => {
            let _ = fs::remove_dir_all(&target);
            Err(ProjectError {
                code: "operation_cancelled",
                message: "candidate validation was cancelled".to_owned(),
                diagnostics: Vec::new(),
            })
        }
        Err(ProcessFailure::Failed(error)) => {
            let _ = fs::remove_dir_all(&target);
            Ok(CandidateValidation::Failed {
                diagnostics: vec![error],
            })
        }
    }
}
