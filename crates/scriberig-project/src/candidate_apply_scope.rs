//! Selects repository paths included in candidate refresh and conflict checks.

use std::collections::BTreeSet;
use std::path::Path;

use crate::{ProjectError, ProjectSourceEffect};
use scriberig_store_sqlite::ProjectFileRecord;

/// Selects the working-tree path set for candidate preflight and refresh.
///
/// # Parameters
/// - `root`: Authorized project checkout.
/// - `base_files`: Exact project manifest at candidate base.
/// - `effects`: Candidate-owned paths, including new source files.
/// - `super`: Parent candidate module.
///
/// # Returns
/// Git working paths plus base/effect paths for repositories; `None` for
/// non-Git projects, whose full source trees remain the scan boundary.
///
/// # Errors
/// Returns a structured Git discovery error.
pub(super) fn candidate_scan_paths(
    root: &Path,
    base_files: &[ProjectFileRecord],
    effects: &[ProjectSourceEffect],
) -> Result<Option<BTreeSet<String>>, ProjectError> {
    if !root.join(".git").exists() {
        return Ok(None);
    }
    let mut paths = crate::support::git_candidate_paths(root).map_err(|error| ProjectError {
        code: "git_scan_failed",
        message: error.to_string(),
        diagnostics: Vec::new(),
    })?;
    paths.extend(base_files.iter().map(|file| file.relative_path.clone()));
    paths.extend(effects.iter().map(|effect| effect.path.clone()));
    Ok(Some(paths))
}
