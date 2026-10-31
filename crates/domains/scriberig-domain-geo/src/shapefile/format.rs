//! Parses the deterministic Shapefile and dBase subset without native handles.

use std::collections::BTreeMap;

use scriberig_core::EntityId;

use crate::{CoordinateReferenceSystem, Field, FieldType, GeoValue, GeometryType};

use super::ShapefileError;

mod shape_records;
pub(super) use shape_records::{geometry, shape_records};

/// Describes one dBase column and its row byte location.
#[derive(Clone, Debug)]
pub(super) struct DbfColumn {
    /// Authored field name.
    name: String,
    /// dBase type code.
    kind: u8,
    /// Width in bytes.
    width: usize,
    /// Offset in a row after the deletion marker.
    offset: usize,
}

/// Stores dBase schema and record byte boundaries.
#[derive(Clone, Debug)]
pub(super) struct DbfHeader {
    /// Number of dBase records.
    pub(super) records: usize,
    /// Record data start in bytes.
    header_length: usize,
    /// Full row width including deletion marker.
    record_length: usize,
    /// Parsed columns in authored order.
    columns: Vec<DbfColumn>,
    /// Provider-neutral field definitions.
    pub(super) fields: Vec<Field>,
}

/// Parses one dBase header and its column descriptors.
///
/// # Parameters
/// - `bytes`: Exact `.dbf` member bytes.
///
/// # Returns
/// Record count, row layout, schema fields, and column offsets.
///
/// # Errors
/// Returns `InvalidAttributes` for an unsupported or malformed dBase schema.
pub(super) fn dbf_header(bytes: &[u8]) -> Result<DbfHeader, ShapefileError> {
    if bytes.len() < 33 {
        return Err(ShapefileError::InvalidAttributes);
    }
    let records =
        usize::try_from(le_u32(bytes, 4)?).map_err(|_| ShapefileError::InvalidAttributes)?;
    let header_length = usize::from(le_u16(bytes, 8)?);
    let record_length = usize::from(le_u16(bytes, 10)?);
    if header_length < 33 || header_length > bytes.len() || record_length < 1 {
        return Err(ShapefileError::InvalidAttributes);
    }
    let mut columns = Vec::new();
    let mut fields = Vec::new();
    let mut descriptor = 32;
    let mut offset = 0;
    while descriptor < header_length && bytes[descriptor] != 0x0d {
        let entry = bytes
            .get(descriptor..descriptor + 32)
            .ok_or(ShapefileError::InvalidAttributes)?;
        let name_end = entry[..11].iter().position(|byte| *byte == 0).unwrap_or(11);
        let name = std::str::from_utf8(&entry[..name_end])
            .map_err(|_| ShapefileError::InvalidAttributes)?
            .trim()
            .to_owned();
        let kind = entry[11];
        let width = usize::from(entry[16]);
        if name.is_empty() || width == 0 {
            return Err(ShapefileError::InvalidAttributes);
        }
        let field_type = match kind {
            b'C' => FieldType::Text,
            b'N' | b'F' => FieldType::Number,
            b'L' => FieldType::Boolean,
            b'D' => FieldType::DateTime,
            _ => return Err(ShapefileError::InvalidAttributes),
        };
        columns.push(DbfColumn {
            name: name.clone(),
            kind,
            width,
            offset,
        });
        fields.push(Field {
            name,
            field_type,
            nullable: true,
        });
        offset = offset
            .checked_add(width)
            .ok_or(ShapefileError::InvalidAttributes)?;
        descriptor += 32;
    }
    if descriptor >= header_length || offset + 1 != record_length {
        return Err(ShapefileError::InvalidAttributes);
    }
    let content_end = header_length
        .checked_add(
            records
                .checked_mul(record_length)
                .ok_or(ShapefileError::InvalidAttributes)?,
        )
        .ok_or(ShapefileError::InvalidAttributes)?;
    if content_end > bytes.len() {
        return Err(ShapefileError::InvalidAttributes);
    }
    Ok(DbfHeader {
        records,
        header_length,
        record_length,
        columns,
        fields,
    })
}

/// Decodes typed values from one exact dBase row.
///
/// # Parameters
/// - `bytes`: Exact `.dbf` member bytes.
/// - `header`: Previously validated schema and row layout.
/// - `index`: Zero-based row index matching the Shapefile record order.
///
/// # Returns
/// Attribute map keyed by authored dBase field name.
///
/// # Errors
/// Returns `InvalidAttributes` for malformed records or scalar values.
pub(super) fn attributes(
    bytes: &[u8],
    header: &DbfHeader,
    index: usize,
) -> Result<BTreeMap<String, GeoValue>, ShapefileError> {
    if index >= header.records {
        return Err(ShapefileError::InvalidAttributes);
    }
    let start = header
        .header_length
        .checked_add(
            index
                .checked_mul(header.record_length)
                .ok_or(ShapefileError::InvalidAttributes)?,
        )
        .ok_or(ShapefileError::InvalidAttributes)?;
    let row = bytes
        .get(start..start + header.record_length)
        .ok_or(ShapefileError::InvalidAttributes)?;
    if row[0] != b' ' {
        return Err(ShapefileError::InvalidAttributes);
    }
    let mut attributes = BTreeMap::new();
    for column in &header.columns {
        let raw = row
            .get(1 + column.offset..1 + column.offset + column.width)
            .ok_or(ShapefileError::InvalidAttributes)?;
        let text = std::str::from_utf8(raw)
            .map_err(|_| ShapefileError::InvalidAttributes)?
            .trim();
        let value = if text.is_empty() || text == "?" {
            GeoValue::Null
        } else {
            match column.kind {
                b'C' | b'M' => GeoValue::Text(text.to_owned()),
                b'N' | b'F' => {
                    if !valid_decimal(text) {
                        return Err(ShapefileError::InvalidAttributes);
                    }
                    GeoValue::Number(text.to_owned())
                }
                b'L' => match text.to_ascii_uppercase().as_str() {
                    "T" | "Y" => GeoValue::Boolean(true),
                    "F" | "N" => GeoValue::Boolean(false),
                    _ => GeoValue::Null,
                },
                b'D' => GeoValue::DateTime(text.to_owned()),
                _ => return Err(ShapefileError::InvalidAttributes),
            }
        };
        attributes.insert(column.name.clone(), value);
    }
    Ok(attributes)
}

/// Reports whether one DBF row is marked as deleted.
///
/// # Parameters
/// - `bytes`: Exact `.dbf` member bytes.
/// - `header`: Previously validated schema and row layout.
/// - `index`: Zero-based row index matching the Shapefile record order.
///
/// # Returns
/// `true` when the row carries the dBase deletion marker.
///
/// # Errors
/// Returns `InvalidAttributes` for malformed rows or unknown deletion flags.
pub(super) fn is_deleted(
    bytes: &[u8],
    header: &DbfHeader,
    index: usize,
) -> Result<bool, ShapefileError> {
    if index >= header.records {
        return Err(ShapefileError::InvalidAttributes);
    }
    let offset = header
        .header_length
        .checked_add(
            index
                .checked_mul(header.record_length)
                .ok_or(ShapefileError::InvalidAttributes)?,
        )
        .ok_or(ShapefileError::InvalidAttributes)?;
    match bytes.get(offset).copied() {
        Some(b' ') => Ok(false),
        Some(b'*') => Ok(true),
        _ => Err(ShapefileError::InvalidAttributes),
    }
}

/// Derives a stable layer identity from dataset identity and the authored stem.
///
/// # Parameters
/// - `dataset`: Stable semantic dataset identity.
/// - `name`: Exact authored filename stem.
///
/// # Returns
/// Stable semantic layer identity independent of source record ordering.
pub(super) fn semantic_layer_id(dataset: EntityId, name: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-geo-shapefile-layer-v1\0");
    hasher.update(dataset.as_bytes());
    hasher.update(name.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Maps supported source shape types to provider-neutral geometry semantics.
///
/// # Parameters
/// - `shape_type`: ESRI Shapefile type code.
///
/// # Returns
/// Semantic geometry kind, or `None` for Null Shape.
///
/// # Errors
/// Returns `UnsupportedShapeType` for unsupported source geometry models.
pub(super) fn geometry_type(shape_type: i32) -> Result<Option<GeometryType>, ShapefileError> {
    match shape_type {
        0 => Ok(None),
        1 => Ok(Some(GeometryType::Point)),
        3 | 8 => Ok(Some(GeometryType::GeometryCollection)),
        5 => Ok(Some(GeometryType::Polygon)),
        _ => Err(ShapefileError::UnsupportedShapeType),
    }
}

/// Decodes a supported `.prj` member as explicit source CRS evidence.
///
/// # Parameters
/// - `bytes`: Exact projection sidecar bytes.
///
/// # Returns
/// CRS with the exact WKT definition retained.
///
/// # Errors
/// Returns `InvalidAttributes` for empty or non-UTF-8 source content.
pub(super) fn projection_crs(bytes: &[u8]) -> Result<CoordinateReferenceSystem, ShapefileError> {
    let definition = std::str::from_utf8(bytes)
        .map_err(|_| ShapefileError::InvalidAttributes)?
        .trim();
    if definition.is_empty() {
        return Err(ShapefileError::InvalidAttributes);
    }
    Ok(CoordinateReferenceSystem {
        authority: None,
        code: None,
        definition: Some(definition.to_owned()),
    })
}

/// Validates a plain-decimal dBase numeric scalar.
///
/// # Parameters
/// - `value`: Trimmed source text from a numeric DBF field.
///
/// # Returns
/// Whether the spelling is a signed plain decimal without exponent notation.
fn valid_decimal(value: &str) -> bool {
    let value = value.strip_prefix(['-', '+']).unwrap_or(value);
    let mut pieces = value.split('.');
    let integer = pieces.next().unwrap_or_default();
    let fraction = pieces.next();
    !integer.is_empty()
        && integer.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.is_none_or(|digits| digits.bytes().all(|byte| byte.is_ascii_digit()))
        && pieces.next().is_none()
}

/// Reads a little-endian signed 32-bit field.
///
/// # Parameters
/// - `bytes`: Source buffer.
/// - `offset`: Byte offset of the field.
///
/// # Returns
/// Decoded signed integer.
///
/// # Errors
/// Returns `InvalidShape` when the buffer is truncated.
fn le_i32(bytes: &[u8], offset: usize) -> Result<i32, ShapefileError> {
    read_array(bytes, offset).map(i32::from_le_bytes)
}

/// Reads a little-endian unsigned 32-bit field.
///
/// # Parameters
/// - `bytes`: Source buffer.
/// - `offset`: Byte offset of the field.
///
/// # Returns
/// Decoded unsigned integer.
///
/// # Errors
/// Returns `InvalidShape` when the buffer is truncated.
fn le_u32(bytes: &[u8], offset: usize) -> Result<u32, ShapefileError> {
    read_array(bytes, offset).map(u32::from_le_bytes)
}

/// Reads a little-endian unsigned 16-bit field.
///
/// # Parameters
/// - `bytes`: Source buffer.
/// - `offset`: Byte offset of the field.
///
/// # Returns
/// Decoded unsigned integer.
///
/// # Errors
/// Returns `InvalidShape` when the buffer is truncated.
fn le_u16(bytes: &[u8], offset: usize) -> Result<u16, ShapefileError> {
    read_array(bytes, offset).map(u16::from_le_bytes)
}

/// Copies one fixed-width field into a byte array.
///
/// # Parameters
/// - `bytes`: Source buffer.
/// - `offset`: Byte offset of the field.
///
/// # Returns
/// Exact fixed-width byte array.
///
/// # Errors
/// Returns `InvalidShape` when the buffer is truncated.
fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], ShapefileError> {
    bytes
        .get(offset..offset + N)
        .and_then(|slice| slice.try_into().ok())
        .ok_or(ShapefileError::InvalidShape)
}
