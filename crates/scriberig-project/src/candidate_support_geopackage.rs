//! Validates staged `GeoPackage` packages against exact base feature semantics.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{ProcessFailure, safe_join};

/// Reopens each changed `GeoPackage` and validates one geometry or attribute edit.
///
/// # Parameters
/// - `root`: Isolated candidate filesystem root.
/// - `effects`: Exact project source effects staged for validation.
/// - `cancelled`: Shared cancellation flag for the validation operation.
///
/// # Returns
/// Empty diagnostics on success, or bounded package validation failures.
///
/// # Errors
/// Returns cancellation or isolated file-access failures.
pub(super) fn validate_geopackage_packages(
    root: &Path,
    effects: &[super::super::ProjectSourceEffect],
    cancelled: &AtomicBool,
) -> Result<Vec<String>, ProcessFailure> {
    let packages = effects
        .iter()
        .filter(|effect| {
            Path::new(&effect.path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("gpkg"))
        })
        .collect::<Vec<_>>();
    if packages.is_empty() {
        return Ok(vec!["candidate has no GeoPackage to validate".into()]);
    }
    let mut diagnostics = Vec::new();
    for effect in packages {
        if cancelled.load(Ordering::Acquire) {
            return Err(ProcessFailure::Cancelled);
        }
        if effect.kind != super::super::ProjectSourceEffectKind::Update {
            diagnostics.push(format!(
                "{} GeoPackage changes must update an existing package",
                effect.path
            ));
            continue;
        }
        let (Some(before), Some(expected)) = (effect.old.as_deref(), effect.new.as_deref()) else {
            diagnostics.push(format!(
                "{} has no exact base or staged GeoPackage bytes",
                effect.path
            ));
            continue;
        };
        let path =
            safe_join(root, &effect.path).map_err(|error| ProcessFailure::Failed(error.message))?;
        let actual = fs::read(path).map_err(|error| ProcessFailure::Failed(error.to_string()))?;
        if actual.as_slice() != expected {
            diagnostics.push(format!(
                "{} staged bytes do not match the exact candidate effect",
                effect.path
            ));
            continue;
        }
        if scriberig_domain_geo::validate_geopackage_feature_edit(before, &actual).is_err() {
            diagnostics.push(format!(
                "{} failed single-feature geometry or attribute validation",
                effect.path
            ));
        }
    }
    Ok(diagnostics)
}
