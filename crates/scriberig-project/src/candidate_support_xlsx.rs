//! Validates staged XLSX packages inside isolated candidate workspaces.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{ProcessFailure, safe_join};

/// Reopens each changed XLSX package staged in the isolated candidate root.
///
/// # Parameters
/// - `root`: Isolated candidate filesystem root.
/// - `effects`: Exact candidate source effects staged into that root.
/// - `cancelled`: Shared cancellation flag for the validation operation.
///
/// # Returns
/// Empty diagnostics on success, or bounded package validation failures.
///
/// # Errors
/// Returns cancellation or safe file-access errors.
pub(super) fn validate_xlsx_packages(
    root: &Path,
    effects: &[super::super::ProjectSourceEffect],
    cancelled: &AtomicBool,
) -> Result<Vec<String>, ProcessFailure> {
    let paths = effects
        .iter()
        .filter(|effect| {
            Path::new(&effect.path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
        })
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Ok(vec!["candidate has no XLSX package to validate".into()]);
    }
    let mut diagnostics = Vec::new();
    for effect in paths {
        if cancelled.load(Ordering::Acquire) {
            return Err(ProcessFailure::Cancelled);
        }
        let Some(old) = effect.old.as_deref() else {
            diagnostics.push(format!(
                "{} has no exact base package for semantic validation",
                effect.path
            ));
            continue;
        };
        if effect.kind != super::super::ProjectSourceEffectKind::Update {
            diagnostics.push(format!(
                "{} XLSX package changes must update an existing package",
                effect.path
            ));
            continue;
        }
        let path =
            safe_join(root, &effect.path).map_err(|error| ProcessFailure::Failed(error.message))?;
        let bytes = fs::read(path).map_err(|error| ProcessFailure::Failed(error.to_string()))?;
        if effect.new.as_deref() != Some(bytes.as_slice()) {
            diagnostics.push(format!(
                "{} staged bytes do not match the exact candidate effect",
                effect.path
            ));
            continue;
        }
        if let Err(error) = scriberig_domain_xlsx::validate_edit(old, &bytes) {
            diagnostics.push(format!("{} failed validation: {error}", effect.path));
        }
    }
    Ok(diagnostics)
}
