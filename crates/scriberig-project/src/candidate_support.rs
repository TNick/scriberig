//! Provides candidate identity, path, and isolated validation helpers.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

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
        let mut command = Command::new("cargo");
        command
            .args([
                "check",
                "--workspace",
                "--all-targets",
                "--offline",
                "--manifest-path",
            ])
            .arg(manifest)
            .env("CARGO_NET_OFFLINE", "true")
            .env(
                "CARGO_TARGET_DIR",
                std::env::temp_dir().join("scriberig-candidate-validation-target"),
            );
        let output = run_bounded(command, Duration::from_secs(600))?;
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

/// Runs a validation command with a deadline and bounded captured output.
///
/// # Parameters
/// - `command`: Configured validation process.
/// - `timeout`: Maximum process duration.
///
/// # Returns
/// Process output with each stream limited to 64 KiB.
///
/// # Errors
/// Returns a structured message for startup, timeout, or output-reader failure.
fn run_bounded(mut command: Command, timeout: Duration) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("start failure: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "stdout pipe unavailable".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "stderr pipe unavailable".to_owned())?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr));
    let status = wait_bounded(&mut child, timeout)?;
    let stdout = stdout_reader
        .join()
        .map_err(|_| "stdout reader failed".to_owned())??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "stderr reader failed".to_owned())??;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Waits for a child process and terminates it when its deadline expires.
///
/// # Parameters
/// - `child`: Running validation process.
/// - `timeout`: Maximum process duration.
///
/// # Returns
/// The completed process status.
///
/// # Errors
/// Returns a timeout or process polling failure.
fn wait_bounded(child: &mut Child, timeout: Duration) -> Result<ExitStatus, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("timeout after {} seconds", timeout.as_secs()));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Drains a process stream while retaining only a bounded prefix.
///
/// # Parameters
/// - `reader`: Child process output stream.
///
/// # Returns
/// At most 64 KiB of output.
///
/// # Errors
/// Returns a stream read failure.
fn read_bounded(mut reader: impl Read) -> Result<Vec<u8>, String> {
    let mut retained = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(retained);
        }
        let remaining = 65_536_usize.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..count.min(remaining)]);
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

#[cfg(all(test, unix))]
mod tests {
    use super::run_bounded;
    use std::process::Command;
    use std::time::Duration;

    /// Verifies that validation processes are terminated at their deadline.
    #[test]
    fn validation_process_timeout_is_reported() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 2"]);
        let error = run_bounded(command, Duration::from_millis(20))
            .expect_err("slow process must time out");
        assert!(error.starts_with("timeout after"), "{error}");
    }
}
