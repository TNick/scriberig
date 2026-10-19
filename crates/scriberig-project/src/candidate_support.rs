//! Provides candidate identity, path, and isolated validation helpers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store_sqlite::ProjectFileRecord;

use crate::{CandidateValidation, CandidateValidationProfile, ProjectError};

#[path = "candidate_validation_process.rs"]
mod validation_process;
use validation_process::{ProcessFailure, bounded_tail, run_bounded};

/// Builds a stable candidate identity from exact request inputs.
///
/// - `project`: Project identity.
/// - `snapshot`: Exact base snapshot.
/// - `entity`: Selected semantic entity.
/// - `text`: Complete candidate source.
/// - `super`: Parent module visibility.
///
/// Stable candidate identifier.
/// # Returns
/// The operation result.
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
/// - `root`: Authorized project root.
/// - `relative`: Project-relative path.
/// - `super`: Parent module visibility.
///
/// Safe joined path.
///
/// Returns an error when the path is absolute or escapes the root.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
    let path = root.join(relative);
    crate::project_security::require_public_path(&path)?;
    Ok(path)
}

/// Validates a candidate in an isolated copy of the project.
///
/// - `root`: Project root to copy.
/// - `relative`: Candidate-relative file path.
/// - `text`: Candidate source text.
/// - `super`: Parent module visibility.
///
/// Semantic and Cargo validation state.
/// # Returns
/// The operation result.
pub(super) fn validate_project(root: &Path, relative: &str, text: &str) -> CandidateValidation {
    let Ok(temporary_dir) = tempfile::tempdir() else {
        return CandidateValidation::Failed {
            diagnostics: vec!["isolated validation directory unavailable".to_owned()],
        };
    };
    let temporary = temporary_dir.path();
    let result = (|| {
        let sensitive = scriberig_secrets::sensitive::SensitivePaths::from_process()
            .map_err(|_| "secret exclusion policy unavailable".to_owned())?;
        copy_tree(root, temporary, &sensitive)?;
        let path = safe_join(temporary, relative).map_err(|error| error.message)?;
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
        scriberig_secrets::process::prepare_command(&mut command);
        if use_rustup {
            command.args(["run", "stable", "cargo", "check"]);
        } else {
            command.arg("check");
        }
        let output = command
            .current_dir(temporary)
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
/// - `source`: Project root to copy.
/// - `destination`: Isolated destination tree.
/// - `sensitive`: Secret-file exclusions read before copying the project.
///
/// Unit after the copy completes.
///
/// Returns an error when a filesystem operation fails.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn copy_tree(
    source: &Path,
    destination: &Path,
    sensitive: &scriberig_secrets::sensitive::SensitivePaths,
) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        if sensitive.contains(&source_path) {
            continue;
        }
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            copy_tree(&source_path, &destination_path, sensitive)?;
        } else if file_type.is_file() {
            fs::copy(source_path, destination_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

/// Validates a source-effect set in an isolated tree reconstructed from base bytes.
///
/// - `files`: Exact project source manifest for the candidate base.
/// - `effects`: Ordered candidate source effects.
/// - `candidate_id`: Stable identity used to isolate the validation workspace.
/// - `profile`: Validation commands and bounds selected for this project.
/// - `cancelled`: Shared cancellation flag for the owning operation.
/// - `super`: Parent candidate module visibility.
///
/// Candidate validation state with bounded process diagnostics.
///
/// Returns cancellation when validation was stopped by its owning operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
        run_candidate_cargo(&root, &target, profile, cancelled)
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

/// Runs one registered Cargo profile against an isolated candidate root.
///
/// - `root`: Materialized candidate workspace root.
/// - `target`: Candidate-specific Cargo artifact directory.
/// - `profile`: Application-selected Cargo policy.
/// - `cancelled`: Shared operation cancellation flag.
///
/// Empty diagnostics on success or bounded failure details.
///
/// Returns when execution is cancelled or process setup fails.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn run_candidate_cargo(
    root: &Path,
    target: &Path,
    profile: CandidateValidationProfile,
    cancelled: &AtomicBool,
) -> Result<Vec<String>, ProcessFailure> {
    let manifest = root.join("Cargo.toml");
    if !manifest.exists() {
        return Ok(Vec::new());
    }
    if profile.allows_dependency_acquisition() {
        if !root.join("Cargo.lock").is_file() {
            return Ok(vec![
                "dependency resolution requires the exact candidate Cargo.lock".to_owned(),
            ]);
        }
        if root.join(".cargo/config").exists() || root.join(".cargo/config.toml").exists() {
            return Ok(vec![
                "dependency resolution rejects candidate-local Cargo configuration".to_owned(),
            ]);
        }
    }
    let mut command = Command::new("cargo");
    command
        .args(profile.cargo_arguments())
        .arg("--manifest-path")
        .arg(manifest)
        .current_dir(root)
        .env(
            "CARGO_NET_OFFLINE",
            if profile.allows_dependency_acquisition() {
                "false"
            } else {
                "true"
            },
        )
        .env("CARGO_TARGET_DIR", target);
    #[cfg(windows)]
    command.env("CARGO_PROFILE_TEST_DEBUG", "0");
    if profile.allows_dependency_acquisition() {
        command
            .env("CARGO_NET_RETRY", "1")
            .env("CARGO_HTTP_TIMEOUT", "30");
    }
    let output = run_bounded(command, profile.timeout(), cancelled)?;
    if output.status.success() {
        return Ok(Vec::new());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let diagnostics = if stderr.trim().is_empty() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        stderr.into_owned()
    };
    Ok(vec![bounded_tail(&diagnostics, 16_384)])
}

/// Resolves the isolated Cargo target cache for one exact candidate.
///
/// - `candidate_id`: Stable candidate identity.
///
/// Candidate-specific build artifact directory.
///
/// Uses `CARGO_TARGET_DIR` when configured and otherwise uses the process temp
/// directory.
/// # Returns
/// The operation result.
fn candidate_target(candidate_id: &str) -> PathBuf {
    let workspace_id = blake3::hash(candidate_id.as_bytes()).to_hex().to_string();
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map_or_else(std::env::temp_dir, |path| {
            if path.is_absolute() {
                path
            } else {
                std::env::current_dir()
                    .map_or_else(|_| std::env::temp_dir(), |directory| directory.join(path))
            }
        });
    root.join(format!("scriberig-candidate-target-{workspace_id}"))
}

/// Removes cached build artifacts after a candidate reaches a terminal Apply outcome.
///
/// - `candidate_id`: Stable candidate identity whose target cache can be released.
/// - `super`: Parent candidate module visibility marker.
pub(super) fn cleanup_candidate_target(candidate_id: &str) {
    let _ = fs::remove_dir_all(candidate_target(candidate_id));
}

/// Writes exact candidate bytes beneath an isolated workspace root.
///
/// - `root`: Isolated candidate workspace.
/// - `relative`: Project-relative path.
/// - `bytes`: Exact source bytes.
///
/// Unit after the candidate path is materialized.
///
/// Returns an error for an unsafe path or failed filesystem operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
