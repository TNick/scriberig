//! Validates staged QGIS project rename candidates in isolated workspaces.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{ProcessFailure, safe_join};

/// Reopens and validates each exact QGS/QGZ semantic edit output.
///
/// # Parameters
/// - `root`: Isolated candidate filesystem root.
/// - `effects`: Exact source effects in the candidate.
/// - `semantic_operation`: Exact QGIS edit operation being validated.
/// - `cancelled`: Shared cancellation flag.
///
/// # Returns
/// Empty diagnostics when every effect exactly reproduces the requested safe edit.
///
/// # Errors
/// Returns cancellation or isolated file-access errors.
pub(super) fn validate_qgis_renames(
    root: &Path,
    effects: &[super::super::ProjectSourceEffect],
    semantic_operation: Option<&str>,
    cancelled: &AtomicBool,
) -> Result<Vec<String>, ProcessFailure> {
    let operation = match semantic_operation {
        Some("qgis.rename_layer_reference") => "rename",
        Some("qgis.reorder_project_layers") => "layer order",
        _ => {
            return Ok(vec!["unsupported QGIS semantic operation".into()]);
        }
    };
    if effects.is_empty()
        || effects.iter().any(|effect| {
            !Path::new(&effect.path)
                .extension()
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("qgs") || extension.eq_ignore_ascii_case("qgz")
                })
        })
    {
        return Ok(vec![format!(
            "QGIS {operation} candidates may update only QGS/QGZ files"
        )]);
    }
    let mut diagnostics = Vec::new();
    for effect in effects {
        if cancelled.load(Ordering::Acquire) {
            return Err(ProcessFailure::Cancelled);
        }
        if effect.kind != super::super::ProjectSourceEffectKind::Update {
            diagnostics.push(format!(
                "{} QGIS {operation} must update an existing project",
                effect.path,
            ));
            continue;
        }
        let (Some(before), Some(expected)) = (effect.old.as_deref(), effect.new.as_deref()) else {
            diagnostics.push(format!(
                "{} has no exact base or staged QGIS project bytes",
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
        let validation = match operation {
            "rename" => scriberig_domain_qgis::validate_layer_rename(before, &actual),
            "layer order" => scriberig_domain_qgis::validate_project_layer_order(before, &actual),
            _ => unreachable!("QGIS operation was checked before validation"),
        };
        if let Err(error) = validation {
            diagnostics.push(format!(
                "{} failed QGIS {operation} validation: {error}",
                effect.path,
            ));
        }
    }
    Ok(diagnostics)
}
