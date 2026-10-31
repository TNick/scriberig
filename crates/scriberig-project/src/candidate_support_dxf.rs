//! Validates staged DXF drawings in isolated candidate workspaces.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use scriberig_core::EntityId;
use scriberig_domain_cad::{CadEntity, DrawingEntity};
use scriberig_store_sqlite::ProjectFileRecord;

use super::{ProcessFailure, safe_join};

/// Reopens and validates changed DXF files against their exact source records.
///
/// Complete sources must remain semantically complete. Incomplete sources may
/// be edited only by a targeted CAD operation with an exact allowed-field diff.
///
/// # Parameters
/// - `root`: Isolated candidate filesystem root.
/// - `files`: Exact project files at the candidate base snapshot.
/// - `effects`: Exact source effects staged for validation.
/// - `semantic_operation`: Stable semantic operation label.
/// - `semantic_target`: Semantic entity selected by the operation.
/// - `semantic_changes`: Typed before/after values recorded by the candidate.
/// - `cancelled`: Shared cancellation state for the validator.
///
/// # Returns
/// Empty diagnostics on success or bounded parse/completeness failures.
///
/// # Errors
/// Returns cancellation or isolated file-access failures.
pub(super) fn validate_dxf_packages(
    root: &Path,
    files: &[ProjectFileRecord],
    effects: &[super::super::ProjectSourceEffect],
    semantic_operation: Option<&str>,
    semantic_target: Option<EntityId>,
    semantic_changes: &[super::super::SemanticCandidateChange],
    cancelled: &AtomicBool,
) -> Result<Vec<String>, ProcessFailure> {
    let paths = effects
        .iter()
        .filter(|effect| {
            Path::new(&effect.path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dxf"))
        })
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Ok(vec!["candidate has no DXF drawing to validate".into()]);
    }
    let mut diagnostics = Vec::new();
    for effect in paths {
        if cancelled.load(Ordering::Acquire) {
            return Err(ProcessFailure::Cancelled);
        }
        let path =
            safe_join(root, &effect.path).map_err(|error| ProcessFailure::Failed(error.message))?;
        let candidate =
            fs::read(path).map_err(|error| ProcessFailure::Failed(error.to_string()))?;
        let Some(source) = files
            .iter()
            .find(|file| file.relative_path == effect.path && file.adapter == "dxf")
        else {
            diagnostics.push(format!("{} has no exact DXF base source", effect.path));
            continue;
        };
        let drawing_id = crate::support::adapters::dxf::drawing_id(source.entity);
        let baseline = match scriberig_domain_dxf::import_dxf(&source.bytes, drawing_id) {
            Ok(imported) => imported,
            Err(error) => {
                diagnostics.push(format!("{} base failed to parse: {error}", effect.path));
                continue;
            }
        };
        let updated = match scriberig_domain_dxf::import_dxf(&candidate, drawing_id) {
            Ok(imported) => imported,
            Err(error) => {
                diagnostics.push(format!("{} failed to parse: {error}", effect.path));
                continue;
            }
        };
        if baseline.complete {
            if !updated.complete {
                diagnostics.push(format!(
                    "{} candidate introduced unsupported DXF content",
                    effect.path
                ));
            }
            continue;
        }
        if let Err(error) = validate_opaque_edit(
            &source.bytes,
            &candidate,
            &baseline,
            &updated,
            semantic_operation,
            semantic_target,
            semantic_changes,
        ) {
            diagnostics.push(format!("{}: {}", effect.path, error));
        }
    }
    Ok(diagnostics)
}

/// Verifies an incomplete-source candidate changes only its selected CAD field.
///
/// # Parameters
/// - `baseline_bytes`: Exact original DXF bytes.
/// - `candidate_bytes`: Exact staged DXF bytes.
/// - `baseline`: Semantic import of the original source.
/// - `updated`: Semantic import of the staged source.
/// - `operation`: Stable operation label stored with the candidate.
/// - `target`: Semantic entity selected by the candidate.
/// - `semantic_changes`: Typed before/after values recorded by the candidate.
///
/// # Returns
/// Unit when the operation and raw byte diff prove exact opaque preservation.
///
/// # Errors
/// Returns a safe diagnostic when target, operation, or preservation proof fails.
fn validate_opaque_edit(
    baseline_bytes: &[u8],
    candidate_bytes: &[u8],
    baseline: &scriberig_domain_dxf::DxfImport,
    updated: &scriberig_domain_dxf::DxfImport,
    operation: Option<&str>,
    target: Option<EntityId>,
    semantic_changes: &[super::super::SemanticCandidateChange],
) -> Result<(), String> {
    let target =
        target.ok_or_else(|| "opaque-preserving edits require a semantic target".to_owned())?;
    let operation = operation
        .ok_or_else(|| "opaque-preserving edits require a semantic operation".to_owned())?;
    let before = baseline
        .drawing()
        .entities
        .iter()
        .find(|entity| entity.id == target)
        .ok_or_else(|| "selected target is absent from the exact DXF base".to_owned())?;
    let handle = baseline
        .provider_import
        .entity_identifiers()
        .iter()
        .find(|identifier| identifier.entity == target)
        .map(|identifier| identifier.value.as_str())
        .ok_or_else(|| "selected target has no DXF native handle".to_owned())?;
    let after = updated
        .drawing()
        .entities
        .iter()
        .find(|entity| entity.id == target)
        .ok_or_else(|| "selected target is absent from the staged DXF".to_owned())?;
    let allowed_codes =
        validate_semantic_change(operation, before, after, target, semantic_changes)?;
    scriberig_domain_dxf::verify_opaque_preservation(
        baseline_bytes,
        candidate_bytes,
        handle,
        allowed_codes,
    )
    .map_err(|error| format!("{}: {}", error.code, error.message))
}

/// Binds a candidate operation to its intended semantic field and entity kind.
///
/// # Parameters
/// - `operation`: Stored semantic operation.
/// - `before`: Original imported target.
/// - `after`: Candidate imported target.
///
/// # Returns
/// Group-code fields the exact-byte verifier may permit to change.
///
/// # Errors
/// Returns a structured message when the target semantic diff is invalid.
fn validate_semantic_change<'a>(
    operation: &str,
    before: &'a DrawingEntity,
    after: &'a DrawingEntity,
    target: EntityId,
    changes: &[super::super::SemanticCandidateChange],
) -> Result<&'static [i16], String> {
    if changes.len() != 1 || changes[0].target != target {
        return Err("candidate semantic change does not identify the selected target".to_owned());
    }
    let same_except = |layer, geometry| {
        before.space == after.space
            && before.style == after.style
            && (layer || before.layer == after.layer)
            && (geometry || before.geometry == after.geometry)
    };
    match operation {
        "cad.replace_line_geometry"
            if matches!(before.geometry, CadEntity::Line { .. })
                && matches!(after.geometry, CadEntity::Line { .. })
                && before.geometry != after.geometry
                && same_except(false, true)
                && changes[0].property == "entity.geometry"
                && serde_json::to_value(&before.geometry).ok().as_ref()
                    == Some(&changes[0].before)
                && serde_json::to_value(&after.geometry).ok().as_ref()
                    == Some(&changes[0].after) =>
        {
            Ok(&[10, 20, 30, 11, 21, 31])
        }
        "cad.change_layer"
            if before.layer != after.layer
                && same_except(true, false)
                && changes[0].property == "entity.layer"
                && serde_json::to_value(before.layer).ok().as_ref() == Some(&changes[0].before)
                && serde_json::to_value(after.layer).ok().as_ref() == Some(&changes[0].after) =>
        {
            Ok(&[8])
        }
        "cad.update_text"
            if text_changed_only(before, after)
                && same_except(false, true)
                && changes[0].property == "entity.text"
                && text_value(&before.geometry).as_deref() == changes[0].before.as_str()
                && text_value(&after.geometry).as_deref() == changes[0].after.as_str() =>
        {
            Ok(&[1])
        }
        _ => Err("candidate semantic diff does not match its operation".to_owned()),
    }
}

/// Returns the authored value of a semantic text entity.
///
/// # Parameters
/// - `geometry`: CAD geometry to inspect.
///
/// # Returns
/// Text content when the geometry is an annotation.
fn text_value(geometry: &CadEntity) -> Option<String> {
    match geometry {
        CadEntity::Text { value, .. } => Some(value.clone()),
        _ => None,
    }
}

/// Confirms an imported CAD text target changed only its authored value.
///
/// # Parameters
/// - `before`: Original semantic target.
/// - `after`: Candidate semantic target.
///
/// # Returns
/// Whether text value changed while position and height stayed the same.
fn text_changed_only(before: &DrawingEntity, after: &DrawingEntity) -> bool {
    match (&before.geometry, &after.geometry) {
        (
            CadEntity::Text {
                position: before_position,
                value: before_value,
                height: before_height,
            },
            CadEntity::Text {
                position: after_position,
                value: after_value,
                height: after_height,
            },
        ) => {
            before_position == after_position
                && before_height == after_height
                && before_value != after_value
        }
        _ => false,
    }
}
