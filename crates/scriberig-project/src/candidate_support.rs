//! Provides candidate identity, path, and isolated validation helpers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use crate::{CandidateValidation, CandidateValidationProfile, ProjectError};
use scriberig_core::{EntityId, ProjectId, SnapshotId};

#[path = "candidate_support_docx.rs"]
mod docx_validation;
#[path = "candidate_support_dxf.rs"]
mod dxf_validation;
#[path = "candidate_support/effects_validation.rs"]
mod effects_validation;
#[path = "candidate_support_geopackage.rs"]
mod geopackage_validation;
#[path = "candidate_support_qgis.rs"]
mod qgis_validation;
#[path = "candidate_support_xlsx.rs"]
mod xlsx_validation;
pub(super) use effects_validation::validate_effects;

#[path = "candidate_validation_process.rs"]
mod validation_process;
use validation_process::{ProcessFailure, bounded_tail, run_bounded};

const MAX_CANDIDATE_DIAGNOSTIC_CHARS: usize = 8_000;

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
    if matches!(
        profile,
        CandidateValidationProfile::PhpDomainFocused
            | CandidateValidationProfile::ProjectImportFocused
            | CandidateValidationProfile::ProjectIngestionE2e
    ) {
        command.args(["--", "--nocapture"]);
    }
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
    let mut diagnostics = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !diagnostics.trim().is_empty() {
            diagnostics.push('\n');
        }
        diagnostics.push_str(&stderr);
    }
    Ok(vec![bounded_tail(
        &diagnostics,
        MAX_CANDIDATE_DIAGNOSTIC_CHARS,
    )])
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
