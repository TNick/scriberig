//! Provides candidate identity, path, and isolated validation helpers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store_sqlite::ProjectFileRecord;

use crate::{CandidateValidation, ProjectError};

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
pub(super) fn validate_effects(
    files: &[ProjectFileRecord],
    effects: &[super::ProjectSourceEffect],
) -> CandidateValidation {
    static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);
    let id = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("scriberig-project-validation-{id}"));
    let result = (|| {
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        for file in files {
            write_candidate_file(&root, &file.relative_path, &file.bytes)?;
        }
        for effect in effects {
            let path = safe_join(&root, &effect.path).map_err(|error| error.message)?;
            match &effect.new {
                Some(bytes) => write_candidate_file(&root, &effect.path, bytes)?,
                None => {
                    if path.exists() {
                        fs::remove_file(path).map_err(|error| error.to_string())?;
                    }
                }
            }
        }
        let manifest = root.join("Cargo.toml");
        if !manifest.exists() {
            return Ok(Vec::new());
        }
        let output = Command::new("cargo")
            .args([
                "check",
                "--workspace",
                "--all-targets",
                "--offline",
                "--manifest-path",
            ])
            .arg(manifest)
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_TARGET_DIR", root.join("target"))
            .output()
            .map_err(|error| error.to_string())?;
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
        Ok(diagnostics) if diagnostics.is_empty() => CandidateValidation::Passed,
        Ok(diagnostics) => CandidateValidation::Failed { diagnostics },
        Err(error) => CandidateValidation::Failed {
            diagnostics: vec![error],
        },
    }
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
