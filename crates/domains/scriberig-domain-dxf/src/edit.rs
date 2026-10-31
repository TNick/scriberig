//! Edits supported DXF line geometry while retaining the parsed package model.

use std::io::Cursor;

use scriberig_domain_cad::Coordinate;

#[path = "edit/text.rs"]
mod text;
pub use text::replace_text;
mod raw;
pub use raw::{supports_opaque_preserving_edits, verify_opaque_preservation};

/// Describes a rejected or invalid DXF line edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DxfEditError {
    /// Stable error category for callers.
    pub code: &'static str,
    /// Safe explanation that does not include source bytes.
    pub message: String,
}

/// Replaces one supported LINE entity's endpoints by its native handle.
///
/// The operation accepts only drawings whose supported model-space entities
/// can all be represented by the semantic importer and that contain no
/// classes, blocks, or object records. This deliberately narrow boundary
/// avoids claiming preservation for structures not yet modeled by Scriberig.
/// Coordinates are exact three-dimensional `(x, y, z)` tuples expressed in
/// the source drawing's declared units; the adapter performs no unit conversion.
///
/// # Parameters
/// - `bytes`: Exact source DXF bytes.
/// - `handle`: Native DXF handle retained as reconciliation evidence.
/// - `start`: New exact-text start point, converted to finite DXF coordinates.
/// - `end`: New exact-text end point, converted to finite DXF coordinates.
///
/// # Returns
/// Validated DXF bytes with the selected line geometry changed.
///
/// # Errors
/// Returns an error when the source cannot be safely edited or validation fails.
pub fn replace_line_geometry(
    bytes: &[u8],
    handle: &str,
    start: &Coordinate,
    end: &Coordinate,
) -> Result<Vec<u8>, DxfEditError> {
    let mut drawing = dxf::Drawing::load(&mut Cursor::new(bytes))
        .map_err(|error| edit_error("dxf_parse_failed", format!("could not parse DXF: {error}")))?;
    if ensure_editable_subset(&drawing).is_err() {
        let start = point(start)?;
        let end = point(end)?;
        let output = raw::replace_record_values(
            bytes,
            "LINE",
            handle,
            &[
                (10, start.x.to_string().into_bytes()),
                (20, start.y.to_string().into_bytes()),
                (30, start.z.to_string().into_bytes()),
                (11, end.x.to_string().into_bytes()),
                (21, end.y.to_string().into_bytes()),
                (31, end.z.to_string().into_bytes()),
            ],
        )?;
        validate_replacement(
            &output,
            handle,
            &start,
            &end,
            drawing.header.default_drawing_units,
        )?;
        return Ok(output);
    }
    let start = point(start)?;
    let end = point(end)?;
    let mut matched = false;
    for entity in drawing.entities_mut() {
        if format!("{:X}", entity.common.handle.0) != handle {
            continue;
        }
        let dxf::entities::EntityType::Line(line) = &mut entity.specific else {
            return Err(edit_error(
                "dxf_target_not_line",
                "selected native handle does not identify a LINE entity",
            ));
        };
        line.p1 = start.clone();
        line.p2 = end.clone();
        matched = true;
        break;
    }
    if !matched {
        return Err(edit_error(
            "dxf_target_not_found",
            "selected native handle is not present in the drawing",
        ));
    }
    let mut output = Vec::new();
    drawing.save(&mut output).map_err(|error| {
        edit_error(
            "dxf_render_failed",
            format!("could not render DXF: {error}"),
        )
    })?;
    validate_replacement(
        &output,
        handle,
        &start,
        &end,
        drawing.header.default_drawing_units,
    )?;
    Ok(output)
}

/// Moves one supported DXF entity to an existing named layer.
///
/// The operation only changes the selected entity's layer reference. It rejects
/// drawings with unmodeled records so source details cannot be silently lost.
///
/// # Parameters
/// - `bytes`: Exact source DXF bytes.
/// - `handle`: Native handle retained for the selected semantic entity.
/// - `destination_layer`: Existing DXF layer name.
///
/// # Returns
/// Reopened DXF bytes with the selected entity assigned to the destination layer.
///
/// # Errors
/// Returns an error when the source is unsupported, the layer or entity is
/// missing, or output validation fails.
pub fn change_entity_layer(
    bytes: &[u8],
    handle: &str,
    destination_layer: &str,
) -> Result<Vec<u8>, DxfEditError> {
    let mut drawing = dxf::Drawing::load(&mut Cursor::new(bytes))
        .map_err(|error| edit_error("dxf_parse_failed", format!("could not parse DXF: {error}")))?;
    if ensure_editable_subset(&drawing).is_err() {
        if !drawing
            .layers()
            .any(|layer| layer.name == destination_layer)
        {
            return Err(edit_error(
                "dxf_layer_not_found",
                "destination DXF layer is not present in the drawing",
            ));
        }
        let entity_type = drawing
            .entities()
            .find(|entity| format!("{:X}", entity.common.handle.0) == handle)
            .and_then(|entity| entity_type_name(&entity.specific))
            .ok_or_else(|| {
                edit_error(
                    "dxf_edit_unsupported",
                    "selected native handle is not a supported entity type",
                )
            })?;
        if !destination_layer.is_ascii() {
            return Err(edit_error(
                "dxf_edit_opaque_encoding_unsupported",
                "opaque-preserving layer edits require an ASCII layer name",
            ));
        }
        let output = raw::replace_record_values(
            bytes,
            entity_type,
            handle,
            &[(8, destination_layer.as_bytes().to_vec())],
        )?;
        validate_layer_change(&output, handle, destination_layer)?;
        return Ok(output);
    }
    if !drawing
        .layers()
        .any(|layer| layer.name == destination_layer)
    {
        return Err(edit_error(
            "dxf_layer_not_found",
            "destination DXF layer is not present in the drawing",
        ));
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
    destination_layer.clone_into(&mut entity.common.layer);
    let mut output = Vec::new();
    drawing.save(&mut output).map_err(|error| {
        edit_error(
            "dxf_render_failed",
            format!("could not render DXF: {error}"),
        )
    })?;
    validate_layer_change(&output, handle, destination_layer)?;
    Ok(output)
}

/// Checks that a parsed entity belongs to the semantic importer subset.
///
/// # Parameters
/// - `entity`: Parser-owned DXF entity being inspected.
///
/// # Returns
/// Whether the entity can be represented by the semantic importer.
fn supported_entity(entity: &dxf::entities::Entity) -> bool {
    entity_type_name(&entity.specific).is_some()
}

/// Maps semantic DXF entities to their native record type names.
///
/// # Parameters
/// - `entity`: Parsed DXF entity kind.
///
/// # Returns
/// The native record name when the semantic importer supports the entity.
fn entity_type_name(entity: &dxf::entities::EntityType) -> Option<&'static str> {
    Some(match entity {
        dxf::entities::EntityType::Line(_) => "LINE",
        dxf::entities::EntityType::Polyline(_) => "POLYLINE",
        dxf::entities::EntityType::LwPolyline(_) => "LWPOLYLINE",
        dxf::entities::EntityType::Circle(_) => "CIRCLE",
        dxf::entities::EntityType::Arc(_) => "ARC",
        dxf::entities::EntityType::Text(_) => "TEXT",
        _ => return None,
    })
}

/// Checks whether the parsed drawing fits the complete parser-writer subset.
///
/// # Parameters
/// - `drawing`: Parsed source drawing to inspect.
///
/// # Returns
/// Unit when the complete parsed source fits the supported writer subset.
///
/// # Errors
/// Returns `dxf_edit_unsupported` when classes, blocks, objects, or entity kinds
/// require the byte-preserving raw-record path.
fn ensure_editable_subset(drawing: &dxf::Drawing) -> Result<(), DxfEditError> {
    if drawing.classes.is_empty()
        && drawing.blocks().next().is_none()
        && drawing.objects().next().is_none()
        && drawing.entities().all(supported_entity)
    {
        return Ok(());
    }
    Err(edit_error(
        "dxf_edit_unsupported",
        "drawing contains source records outside the supported editable subset",
    ))
}

/// Converts exact decimal ordinates into finite DXF floating-point coordinates.
///
/// # Parameters
/// - `coordinate`: Exact decimal text supplied by the semantic operation.
///
/// # Returns
/// Parser-native point for DXF output.
///
/// # Errors
/// Returns an error for non-numeric or non-finite ordinates.
fn point(coordinate: &Coordinate) -> Result<dxf::Point, DxfEditError> {
    let parse = |value: &str| {
        value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
    };
    let (Some(x), Some(y), Some(z)) = (
        parse(&coordinate.x),
        parse(&coordinate.y),
        parse(&coordinate.z),
    ) else {
        return Err(edit_error(
            "dxf_invalid_coordinate",
            "line coordinates must be finite numeric values",
        ));
    };
    Ok(dxf::Point::new(x, y, z))
}

/// Reopens output and confirms the selected handle has the requested endpoints.
///
/// # Parameters
/// - `bytes`: Rendered package bytes.
/// - `handle`: Native handle selected by the operation.
/// - `start`: Expected start point.
/// - `end`: Expected end point.
/// - `units`: Declared units from the exact source drawing.
///
/// # Returns
/// Unit when the exact rendered geometry is present.
///
/// # Errors
/// Returns a structured validation error otherwise.
fn validate_replacement(
    bytes: &[u8],
    handle: &str,
    start: &dxf::Point,
    end: &dxf::Point,
    units: dxf::enums::Units,
) -> Result<(), DxfEditError> {
    let drawing = dxf::Drawing::load(&mut Cursor::new(bytes)).map_err(|error| {
        edit_error(
            "dxf_validation_failed",
            format!("rendered DXF did not reopen: {error}"),
        )
    })?;
    let valid = drawing.header.default_drawing_units == units
        && drawing.entities().any(|entity| {
            format!("{:X}", entity.common.handle.0) == handle
                && matches!(&entity.specific,
                    dxf::entities::EntityType::Line(line) if &line.p1 == start && &line.p2 == end)
        });
    if valid {
        Ok(())
    } else {
        Err(edit_error(
            "dxf_validation_failed",
            "rendered DXF does not contain the requested line geometry",
        ))
    }
}

/// Reopens candidate bytes and verifies the target's handle and layer.
///
/// # Parameters
/// - `bytes`: Candidate DXF bytes.
/// - `handle`: Native handle selected by the operation.
/// - `destination_layer`: Expected layer name after rendering.
///
/// # Returns
/// Unit when the selected entity has the requested layer.
///
/// # Errors
/// Returns a validation error when output cannot be reopened or does not match.
fn validate_layer_change(
    bytes: &[u8],
    handle: &str,
    destination_layer: &str,
) -> Result<(), DxfEditError> {
    let drawing = dxf::Drawing::load(&mut Cursor::new(bytes)).map_err(|error| {
        edit_error(
            "dxf_validation_failed",
            format!("rendered DXF did not reopen: {error}"),
        )
    })?;
    let valid = drawing.entities().any(|entity| {
        format!("{:X}", entity.common.handle.0) == handle
            && entity.common.layer == destination_layer
    });
    if valid {
        Ok(())
    } else {
        Err(edit_error(
            "dxf_validation_failed",
            "rendered DXF does not contain the requested entity layer",
        ))
    }
}

/// Creates a safe DXF adapter error.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe description without source bytes.
///
/// # Returns
/// Structured adapter error.
fn edit_error(code: &'static str, message: impl Into<String>) -> DxfEditError {
    DxfEditError {
        code,
        message: message.into(),
    }
}
