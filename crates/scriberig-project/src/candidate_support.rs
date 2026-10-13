//! Provides candidate identity, path, and isolated validation helpers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store_sqlite::ProjectFileRecord;

use crate::{CandidateValidation, CandidateValidationProfile, ProjectError};

#[path = "candidate_validation_process.rs"]
mod validation_process;
use validation_process::{ProcessFailure, run_bounded};

/// Builds a stable candidate identity from exact request inputs.
///
/// # Parameters
/// - `project`: Project identity.
/// - `snapshot`: Exact base snapshot.
/// - `entity`: Selected semantic entity.
/// - `text`: Complete candidate source.
/// - `super`: Parent module visibility.
///
/// # Returns
/// Stable candidate identifier.
pub(super) fn candidate_id(
    project: ProjectId,
    snapshot: SnapshotId,
    entity: EntityId,
    text: &str,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(project.as_bytes());
    hasher.update(snapshot.as_bytes());
    hasher.update(entity.as_bytes());
    hasher.update(text.as_bytes());
    format!("candidate-{}", hasher.finalize().to_hex())
}

/// Restricts a relative project path to its authorized root.
///
/// # Parameters
/// - `root`: Authorized project root.
/// - `relative`: Project-relative path.
/// - `super`: Parent module visibility.
///
/// # Returns
/// Safe joined path.
///
/// # Errors
/// Returns an error when the path is absolute or escapes the root.
pub(super) fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, ProjectError> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ProjectError {
            code: "unsafe_project_path",
            message: relative.display().to_string(),
            diagnostics: Vec::new(),
        });
    }
    Ok(root.join(relative))
}

/// Validates a candidate in an isolated copy of the project.
///
/// # Parameters
/// - `root`: Project root to copy.
/// - `relative`: Candidate-relative file path.
/// - `text`: Candidate source text.
/// - `super`: Parent module visibility.
///
/// # Returns
/// Semantic and Cargo validation state.
pub(super) fn validate_project(root: &Path, relative: &str, text: &str) -> CandidateValidation {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let temporary = std::env::temp_dir().join(format!("scriberig-candidate-{id}"));
    let result = (|| {
        copy_tree(root, &temporary)?;
        let path = safe_join(&temporary, relative).map_err(|error| error.message)?;
        fs::write(path, text).map_err(|error| error.to_string())?;
        let manifest = temporary.join("Cargo.toml");
        if !manifest.exists() {
            return Ok(Vec::new());
        }
        let rustup = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|profile| profile.join(".cargo").join("bin").join("rustup.exe"));
        let use_rustup = rustup.as_ref().is_some_and(|path| path.exists());
        let mut command = rustup
            .filter(|_| use_rustup)
            .map_or_else(|| Command::new("cargo"), Command::new);
        if use_rustup {
            command.args(["run", "stable", "cargo", "check"]);
        } else {
            command.arg("check");
        }
        let output = command
            .args(["--offline", "--manifest-path"])
            .arg(manifest)
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            Ok(Vec::new())
        } else {
            let mut diagnostics = String::from_utf8_lossy(&output.stderr).to_string();
            if diagnostics.trim().is_empty() {
                diagnostics = String::from_utf8_lossy(&output.stdout).to_string();
            }
            Ok(vec![diagnostics])
        }
    })();
    let _ = fs::remove_dir_all(&temporary);
    match result {
        Ok(diagnostics) if diagnostics.is_empty() => CandidateValidation::Passed,
        Ok(diagnostics) => CandidateValidation::Failed { diagnostics },
        Err(error) => CandidateValidation::Failed {
            diagnostics: vec![error],
        },
    }
}

/// Copies a project tree without following directory symlinks.
///
/// # Parameters
/// - `source`: Project root to copy.
/// - `destination`: Isolated destination tree.
///
/// # Returns
/// Unit after the copy completes.
///
/// # Errors
/// Returns an error when a filesystem operation fails.
fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(source_path, destination_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

/// Validates a source-effect set in an isolated tree reconstructed from base bytes.
///
/// # Parameters
/// - `files`: Exact project source manifest for the candidate base.
/// - `effects`: Ordered candidate source effects.
/// - `super`: Parent candidate module visibility.
///
/// # Returns
/// Candidate validation state with bounded process diagnostics.
///
/// # Errors
/// Returns cancellation when validation was stopped by its owning operation.
pub(super) fn validate_effects(
    candidate_id: &str,
    files: &[ProjectFileRecord],
    effects: &[super::ProjectSourceEffect],
    profile: CandidateValidationProfile,
    cancelled: &AtomicBool,
) -> Result<CandidateValidation, ProjectError> {
    let _lock = match super::validation_lock::acquire(cancelled) {
        Ok(guard) => guard,
        Err(super::validation_lock::ValidationLockError::Cancelled) => {
            return Err(ProjectError {
                code: "operation_cancelled",
                message: "candidate validation was cancelled".to_owned(),
                diagnostics: Vec::new(),
            });
        }
        Err(super::validation_lock::ValidationLockError::Failed(error)) => {
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
        let manifest = root.join("Cargo.toml");
        if !manifest.exists() {
            return Ok(Vec::new());
        }
        let mut command = Command::new("cargo");
        command
            .args(profile.cargo_arguments())
            .arg("--manifest-path")
            .arg(manifest)
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_TARGET_DIR", &target);
        let output = run_bounded(command, Duration::from_secs(600), cancelled)?;
        if output.status.success() {
            Ok(Vec::new())
        } else {
            let mut diagnostics = String::from_utf8_lossy(&output.stderr).to_string();
            if diagnostics.trim().is_empty() {
                diagnostics = String::from_utf8_lossy(&output.stdout).to_string();
            }
            Ok(vec![diagnostics.chars().take(16_384).collect()])
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

/// Resolves the isolated Cargo target cache for one exact candidate.
///
/// # Parameters
/// - `candidate_id`: Stable candidate identity.
///
/// # Returns
/// Candidate-specific build artifact directory.
fn candidate_target(candidate_id: &str) -> PathBuf {
    let workspace_id = blake3::hash(candidate_id.as_bytes()).to_hex().to_string();
    std::env::temp_dir().join(format!("scriberig-candidate-target-{workspace_id}"))
}

/// Removes cached build artifacts after a candidate reaches a terminal Apply outcome.
///
/// # Parameters
/// - `candidate_id`: Stable candidate identity whose target cache can be released.
/// - `super`: Parent candidate module visibility marker.
pub(super) fn cleanup_candidate_target(candidate_id: &str) {
    let _ = fs::remove_dir_all(candidate_target(candidate_id));
}

/// Writes exact candidate bytes beneath an isolated workspace root.
///
/// # Parameters
/// - `root`: Isolated candidate workspace.
/// - `relative`: Project-relative path.
/// - `bytes`: Exact source bytes.
///
/// # Returns
/// Unit after the candidate path is materialized.
///
/// # Errors
/// Returns an error for an unsafe path or failed filesystem operation.
fn write_candidate_file(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let path = safe_join(root, relative).map_err(|error| error.message)?;
    let parent = path
        .parent()
        .ok_or_else(|| "candidate path has no parent".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

#[cfg(all(test, unix))]
#[path = "candidate_support_tests.rs"]
mod tests;
