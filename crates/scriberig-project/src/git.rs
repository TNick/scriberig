//! Implements Git-indexed project file discovery.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Reads the currently indexed Git paths for a project root.
///
/// # Parameters
/// - `root`: Project root passed to Git.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Normalized project-relative paths currently tracked by the index.
///
/// # Errors
/// Returns an error when Git is unavailable or the root is not a repository.
pub(crate) fn git_tracked_paths(root: &Path) -> std::io::Result<BTreeSet<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--cached", "-z"])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).replace('\\', "/"))
        .collect())
}

/// Lists tracked and non-ignored untracked repository paths.
///
/// # Parameters
/// - `root`: Project root passed to Git.
/// - `crate`: Internal project support module.
///
/// # Returns
/// Normalized project-relative paths participating in a working-tree refresh.
///
/// # Errors
/// Returns an error when Git is unavailable or the root is not a repository.
pub(crate) fn git_candidate_paths(root: &Path) -> std::io::Result<BTreeSet<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).replace('\\', "/"))
        .collect())
}
