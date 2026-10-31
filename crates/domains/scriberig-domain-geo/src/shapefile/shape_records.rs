//! Parses Shapefile binary framing and supported two-dimensional geometries.

use super::{geometry_type, le_i32, read_array};
use crate::{Coordinate, Geometry, ShapefileError};

/// Validates the Shapefile header and indexes bounded record locations.
///
/// # Parameters
/// - `bytes`: Exact `.shp` member bytes.
///
/// # Returns
/// Global shape type and content offsets/lengths for every record.
///
/// # Errors
/// Returns `InvalidShape` for truncated or inconsistent framing.
pub(in crate::shapefile) fn shape_records(
    bytes: &[u8],
) -> Result<(i32, Vec<(usize, usize)>), ShapefileError> {
    if bytes.len() < 100
        || be_i32(bytes, 0)? != 9994
        || le_i32(bytes, 28)? != 1000
        || usize::try_from(be_i32(bytes, 24)?)
            .ok()
            .and_then(|words| words.checked_mul(2))
            != Some(bytes.len())
    {
        return Err(ShapefileError::InvalidShape);
    }
    let shape_type = le_i32(bytes, 32)?;
    geometry_type(shape_type)?;
    let mut offset = 100;
    let mut records = Vec::new();
    while offset < bytes.len() {
        let record_number = be_i32(bytes, offset)?;
        let length_words = usize::try_from(be_i32(bytes, offset + 4)?)
            .map_err(|_| ShapefileError::InvalidShape)?;
        let length = length_words
            .checked_mul(2)
            .ok_or(ShapefileError::InvalidShape)?;
        let content_start = offset.checked_add(8).ok_or(ShapefileError::InvalidShape)?;
        let content_end = content_start
            .checked_add(length)
            .ok_or(ShapefileError::InvalidShape)?;
        if record_number != i32::try_from(records.len() + 1).unwrap_or(i32::MAX)
            || length < 4
            || content_end > bytes.len()
        {
            return Err(ShapefileError::InvalidShape);
        }
        let record_type = le_i32(bytes, content_start)?;
        if record_type != 0 && record_type != shape_type {
            return Err(ShapefileError::UnsupportedShapeType);
        }
        validate_geometry(
            bytes
                .get(content_start..content_end)
                .ok_or(ShapefileError::InvalidShape)?,
            record_type,
        )?;
        records.push((content_start, length));
        offset = content_end;
    }
    Ok((shape_type, records))
}

/// Decodes one two-dimensional Shapefile geometry record.
///
/// # Parameters
/// - `record`: Record content beginning with the little-endian shape type.
///
/// # Returns
/// Provider-neutral geometry, or `None` for a Null Shape record.
///
/// # Errors
/// Returns `InvalidShape` for malformed coordinate arrays or `UnsupportedShapeType`.
pub(in crate::shapefile) fn geometry(record: &[u8]) -> Result<Option<Geometry>, ShapefileError> {
    let shape_type = le_i32(record, 0)?;
    if shape_type == 0 {
        return Ok(None);
    }
    match shape_type {
        1 => Ok(Some(Geometry::Point(coordinate(record, 4)?))),
        3 | 5 => {
            let parts =
                usize::try_from(le_i32(record, 36)?).map_err(|_| ShapefileError::InvalidShape)?;
            let points =
                usize::try_from(le_i32(record, 40)?).map_err(|_| ShapefileError::InvalidShape)?;
            let part_bytes = parts.checked_mul(4).ok_or(ShapefileError::InvalidShape)?;
            let coordinates_start = 44_usize
                .checked_add(part_bytes)
                .ok_or(ShapefileError::InvalidShape)?;
            let coordinates = read_coordinates(record, coordinates_start, points)?;
            let part_indexes = (0..parts)
                .map(|index| le_i32(record, 44 + index * 4))
                .collect::<Result<Vec<_>, _>>()?;
            let segments = split_parts(&coordinates, &part_indexes)?;
            if shape_type == 3 {
                Ok(Some(Geometry::Collection(
                    segments.into_iter().map(Geometry::LineString).collect(),
                )))
            } else {
                Ok(Some(Geometry::Polygon(segments)))
            }
        }
        8 => {
            let points =
                usize::try_from(le_i32(record, 36)?).map_err(|_| ShapefileError::InvalidShape)?;
            let coordinates = read_coordinates(record, 40, points)?;
            Ok(Some(Geometry::Collection(
                coordinates.into_iter().map(Geometry::Point).collect(),
            )))
        }
        _ => Err(ShapefileError::UnsupportedShapeType),
    }
}

/// Checks record geometry framing without allocating decoded coordinates.
///
/// # Parameters
/// - `record`: Complete record content.
/// - `shape_type`: Record shape type.
///
/// # Returns
/// Unit when the supported record structure is well-formed.
///
/// # Errors
/// Returns `InvalidShape` for malformed framing or `UnsupportedShapeType`.
fn validate_geometry(record: &[u8], shape_type: i32) -> Result<(), ShapefileError> {
    match shape_type {
        0 => Ok(()),
        1 => {
            coordinate(record, 4)?;
            Ok(())
        }
        3 | 5 => {
            let parts =
                usize::try_from(le_i32(record, 36)?).map_err(|_| ShapefileError::InvalidShape)?;
            let points =
                usize::try_from(le_i32(record, 40)?).map_err(|_| ShapefileError::InvalidShape)?;
            let start = 44_usize
                .checked_add(parts.checked_mul(4).ok_or(ShapefileError::InvalidShape)?)
                .ok_or(ShapefileError::InvalidShape)?;
            let expected = start
                .checked_add(points.checked_mul(16).ok_or(ShapefileError::InvalidShape)?)
                .ok_or(ShapefileError::InvalidShape)?;
            if expected != record.len() || (points > 0 && parts == 0) {
                return Err(ShapefileError::InvalidShape);
            }
            let indexes = (0..parts)
                .map(|index| le_i32(record, 44 + index * 4))
                .collect::<Result<Vec<_>, _>>()?;
            let points = i32::try_from(points).map_err(|_| ShapefileError::InvalidShape)?;
            if indexes.first() != Some(&0)
                || indexes.iter().any(|index| *index < 0 || *index >= points)
                || indexes.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(ShapefileError::InvalidShape);
            }
            Ok(())
        }
        8 => {
            let points =
                usize::try_from(le_i32(record, 36)?).map_err(|_| ShapefileError::InvalidShape)?;
            if 40_usize.checked_add(points.checked_mul(16).ok_or(ShapefileError::InvalidShape)?)
                != Some(record.len())
            {
                return Err(ShapefileError::InvalidShape);
            }
            Ok(())
        }
        _ => Err(ShapefileError::UnsupportedShapeType),
    }
}

/// Reads one or more XY coordinate pairs from a record.
///
/// # Parameters
/// - `record`: Complete record content.
/// - `start`: Byte offset of the first coordinate.
/// - `count`: Number of coordinate pairs.
///
/// # Returns
/// Coordinates in source order.
///
/// # Errors
/// Returns `InvalidShape` for truncated or non-finite coordinates.
fn read_coordinates(
    record: &[u8],
    start: usize,
    count: usize,
) -> Result<Vec<Coordinate>, ShapefileError> {
    (0..count)
        .map(|index| coordinate(record, start + index * 16))
        .collect()
}

/// Splits a coordinate vector into validated part ranges.
///
/// # Parameters
/// - `coordinates`: Complete source coordinate sequence.
/// - `starts`: Ordered zero-based part start indexes.
///
/// # Returns
/// Ordered coordinate sequences for each part.
///
/// # Errors
/// Returns `InvalidShape` for an invalid part range.
fn split_parts(
    coordinates: &[Coordinate],
    starts: &[i32],
) -> Result<Vec<Vec<Coordinate>>, ShapefileError> {
    if starts.is_empty() {
        return Ok(Vec::new());
    }
    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let start = usize::try_from(*start).map_err(|_| ShapefileError::InvalidShape)?;
            let end = starts
                .get(index + 1)
                .map_or(Ok(coordinates.len()), |next| {
                    usize::try_from(*next).map_err(|_| ShapefileError::InvalidShape)
                })?;
            coordinates
                .get(start..end)
                .map(<[Coordinate]>::to_vec)
                .ok_or(ShapefileError::InvalidShape)
        })
        .collect()
}

/// Reads one little-endian XY coordinate pair.
///
/// # Parameters
/// - `record`: Complete record content.
/// - `offset`: Byte offset of the x ordinate.
///
/// # Returns
/// Finite coordinates represented as round-trippable decimal text.
///
/// # Errors
/// Returns `InvalidShape` for truncated or non-finite ordinates.
fn coordinate(record: &[u8], offset: usize) -> Result<Coordinate, ShapefileError> {
    let x = f64::from_le_bytes(read_array(record, offset)?);
    let y = f64::from_le_bytes(read_array(record, offset + 8)?);
    if !x.is_finite() || !y.is_finite() {
        return Err(ShapefileError::InvalidShape);
    }
    Ok(Coordinate {
        x: x.to_string(),
        y: y.to_string(),
        z: None,
        measure: None,
    })
}

/// Reads a big-endian signed 32-bit field.
///
/// # Parameters
/// - `bytes`: Source buffer.
/// - `offset`: Byte offset.
///
/// # Returns
/// Decoded signed integer.
///
/// # Errors
/// Returns `InvalidShape` for truncated input.
fn be_i32(bytes: &[u8], offset: usize) -> Result<i32, ShapefileError> {
    read_array(bytes, offset).map(i32::from_be_bytes)
}
