//! Enforces secret-file exclusions for stored manifests and candidate paths.

use crate::ProjectError;
use std::path::Path;

/// Rejects known sensitive and registered credential files before disclosure.
///
/// - `path`: Absolute candidate, projection, or persisted-manifest source path.
/// - `crate`: Internal project-boundary visibility.
///
/// Unit for paths permitted in semantic knowledge.
///
/// Returns a safe secret-boundary denial or unreadable-policy error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn require_public_path(path: &Path) -> Result<(), ProjectError> {
    let policy =
        scriberig_secrets::sensitive::SensitivePaths::from_process().map_err(|_| denied())?;
    if policy.contains(path) {
        return Err(denied());
    }
    Ok(())
}

/// Constructs a safe failure without content from the blocked artifact.
///
/// A structured secret-file boundary error.
/// # Returns
/// The operation result.
fn denied() -> ProjectError {
    ProjectError { code: "sensitive_file_denied", message: "credential files cannot enter project knowledge or candidates; review and rescan the manifest".to_owned(), diagnostics: Vec::new() }
}
