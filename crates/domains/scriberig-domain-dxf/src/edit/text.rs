//! Rewrites one supported DXF text value while preserving its entity properties.

use std::io::Cursor;

use super::{DxfEditError, edit_error, ensure_editable_subset};

/// Replaces the authored text for one supported DXF text entity.
///
/// The writer changes only the text value in the parsed entity. Layer,
/// position, height, style, handle, and every unrelated entity remain intact.
///
/// # Parameters
/// - `bytes`: Exact source DXF bytes.
/// - `handle`: Native handle retained for the selected semantic entity.
/// - `replacement`: New authored text without control characters.
///
/// # Returns
/// Reopened DXF bytes with the selected text value changed.
///
/// # Errors
/// Returns an error for unsupported source records, invalid text, a missing or
/// non-text target, or output validation failure.
pub fn replace_text(
    bytes: &[u8],
    handle: &str,
    replacement: &str,
) -> Result<Vec<u8>, DxfEditError> {
    if replacement.chars().any(char::is_control) {
        return Err(edit_error(
            "dxf_invalid_text",
            "text replacement contains unsupported control characters",
        ));
    }
    let mut drawing = dxf::Drawing::load(&mut Cursor::new(bytes))
        .map_err(|error| edit_error("dxf_parse_failed", format!("could not parse DXF: {error}")))?;
    if ensure_editable_subset(&drawing).is_err() {
        if !replacement.is_ascii() {
            return Err(edit_error(
                "dxf_edit_opaque_encoding_unsupported",
                "opaque-preserving text edits require ASCII replacement text",
            ));
        }
        let entity = drawing
            .entities()
            .find(|entity| format!("{:X}", entity.common.handle.0) == handle)
            .ok_or_else(|| {
                edit_error(
                    "dxf_target_not_found",
                    "selected native handle is not present",
                )
            })?;
        if !matches!(entity.specific, dxf::entities::EntityType::Text(_)) {
            return Err(edit_error(
                "dxf_target_not_text",
                "selected native handle does not identify a text entity",
            ));
        }
        let output = super::raw::replace_record_values(
            bytes,
            "TEXT",
            handle,
            &[(1, replacement.as_bytes().to_vec())],
        )?;
        validate_text_replacement(&output, handle, replacement)?;
        return Ok(output);
    }
    let Some(entity) = drawing
        .entities_mut()
        .find(|entity| format!("{:X}", entity.common.handle.0) == handle)
    else {
        return Err(edit_error(
            "dxf_target_not_found",
            "selected native handle is not present in the drawing",
        ));
    };
    let dxf::entities::EntityType::Text(text) = &mut entity.specific else {
        return Err(edit_error(
            "dxf_target_not_text",
            "selected native handle does not identify a text entity",
        ));
    };
    replacement.clone_into(&mut text.value);
    let mut output = Vec::new();
    drawing.save(&mut output).map_err(|error| {
        edit_error(
            "dxf_render_failed",
            format!("could not render DXF: {error}"),
        )
    })?;
    validate_text_replacement(&output, handle, replacement)?;
    Ok(output)
}

/// Reopens candidate bytes and verifies the selected handle and text value.
///
/// # Parameters
/// - `bytes`: Candidate DXF bytes.
/// - `handle`: Native handle selected by the operation.
/// - `replacement`: Expected authored text after rendering.
///
/// # Returns
/// Unit when the exact target text is present.
///
/// # Errors
/// Returns a validation error when output cannot be reopened or does not match.
fn validate_text_replacement(
    bytes: &[u8],
    handle: &str,
    replacement: &str,
) -> Result<(), DxfEditError> {
    let drawing = dxf::Drawing::load(&mut Cursor::new(bytes)).map_err(|error| {
        edit_error(
            "dxf_validation_failed",
            format!("rendered DXF did not reopen: {error}"),
        )
    })?;
    if drawing.entities().any(|entity| {
        format!("{:X}", entity.common.handle.0) == handle
            && matches!(&entity.specific, dxf::entities::EntityType::Text(text) if text.value == replacement)
    }) {
        Ok(())
    } else {
        Err(edit_error(
            "dxf_validation_failed",
            "rendered DXF does not contain the requested text entity value",
        ))
    }
}
