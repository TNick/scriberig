//! Decodes bounded `GeoPackage` geometry headers and common WKB geometry types.

use std::io::{Cursor, Read};

use crate::{Coordinate, Geometry};

/// Maximum vertices accepted in one geometry row.
const MAX_COORDINATES: u32 = 100_000;
/// Maximum nested WKB geometry depth.
const MAX_DEPTH: usize = 32;

#[cfg(test)]
mod tests;

/// Parses a `GeoPackage` geometry blob into provider-neutral geometry semantics.
///
/// - `bytes`: Exact `GeoPackage` geometry-column blob.
/// - `expected_srs_id`: Optional CRS identifier expected from layer metadata.
/// - `budget`: Aggregate coordinate and geometry-member allocation budget.
///
/// # Returns
/// Semantic geometry, or `None` for the `GeoPackage` empty-geometry marker.
///
/// # Errors
/// Returns a diagnostic for malformed or unsupported binary geometry.
pub(super) fn parse_geopackage_geometry(
    bytes: &[u8],
    expected_srs_id: Option<i32>,
    budget: &mut GeometryBudget,
) -> Result<Option<Geometry>, String> {
    if bytes.len() < 8 || &bytes[..2] != b"GP" || bytes[2] != 0 {
        return Err("invalid GeoPackage geometry header".to_owned());
    }
    let flags = bytes[3];
    let little_endian = flags & 1 != 0;
    let srs_bytes = bytes[4..8]
        .try_into()
        .map_err(|_| "truncated GeoPackage SRS header".to_owned())?;
    let source_srs_id = if little_endian {
        i32::from_le_bytes(srs_bytes)
    } else {
        i32::from_be_bytes(srs_bytes)
    };
    if expected_srs_id.is_some_and(|expected| expected != source_srs_id) {
        return Err("GeoPackage geometry CRS differs from its layer metadata".to_owned());
    }
    let empty = flags & 0x10 != 0;
    let envelope = (flags >> 1) & 7;
    let envelope_values = match envelope {
        0 => 0,
        1 => 4,
        2 | 3 => 6,
        4 => 8,
        _ => return Err("unsupported GeoPackage envelope encoding".to_owned()),
    };
    let wkb_offset = 8 + envelope_values * 8;
    if wkb_offset > bytes.len() {
        return Err("truncated GeoPackage geometry envelope".to_owned());
    }
    if empty {
        return Ok(None);
    }
    let mut cursor = Cursor::new(&bytes[wkb_offset..]);
    let geometry = read_geometry(&mut cursor, 0, budget)?;
    let consumed = usize::try_from(cursor.position())
        .map_err(|_| "WKB geometry length exceeds the supported address space".to_owned())?;
    if consumed != bytes.len() - wkb_offset {
        return Err("trailing bytes in GeoPackage WKB geometry".to_owned());
    }
    Ok(Some(geometry))
}

/// Reads one nested WKB geometry while enforcing the parser depth limit.
///
/// - `cursor`: WKB input cursor.
/// - `depth`: Current collection nesting depth.
/// - `budget`: Aggregate coordinate and member allocation budget.
///
/// # Returns
/// Parsed provider-neutral geometry.
///
/// # Errors
/// Returns a diagnostic for malformed or unsupported WKB.
fn read_geometry(
    cursor: &mut Cursor<&[u8]>,
    depth: usize,
    budget: &mut GeometryBudget,
) -> Result<Geometry, String> {
    if depth > MAX_DEPTH {
        return Err("WKB geometry nesting exceeds the supported depth".to_owned());
    }
    let mut byte_order = [0; 1];
    cursor
        .read_exact(&mut byte_order)
        .map_err(|error| binary_error(&error))?;
    let little = match byte_order[0] {
        0 => false,
        1 => true,
        _ => return Err("invalid WKB byte order".to_owned()),
    };
    let raw_kind = read_u32(cursor, little)?;
    let (kind, has_z, has_m, has_srid) = geometry_kind(raw_kind);
    if has_srid {
        let _ = read_u32(cursor, little)?;
    }
    match kind {
        1 => {
            budget.take_coordinates(1)?;
            Ok(Geometry::Point(read_coordinate(
                cursor, little, has_z, has_m,
            )?))
        }
        2 => {
            let count = read_count(cursor, little)?;
            budget.take_coordinates(count)?;
            let mut points = Vec::with_capacity(count as usize);
            for _ in 0..count {
                points.push(read_coordinate(cursor, little, has_z, has_m)?);
            }
            Ok(Geometry::LineString(points))
        }
        3 => {
            let rings = read_count(cursor, little)?;
            budget.take_members(rings)?;
            let mut polygon = Vec::with_capacity(rings as usize);
            for _ in 0..rings {
                let count = read_count(cursor, little)?;
                budget.take_coordinates(count)?;
                let mut ring = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    ring.push(read_coordinate(cursor, little, has_z, has_m)?);
                }
                polygon.push(ring);
            }
            Ok(Geometry::Polygon(polygon))
        }
        4..=7 => {
            let count = read_count(cursor, little)?;
            budget.take_members(count)?;
            let mut items = Vec::with_capacity(count as usize);
            for _ in 0..count {
                items.push(read_geometry(cursor, depth + 1, budget)?);
            }
            Ok(Geometry::Collection(items))
        }
        _ => Err(format!("unsupported WKB geometry type {kind}")),
    }
}

/// Tracks aggregate allocation bounds across nested geometry values.
pub(super) struct GeometryBudget {
    /// Remaining coordinate tuples allowed in the complete geometry.
    coordinates: usize,
    /// Remaining nested collections/rings allowed in the complete geometry.
    members: usize,
}

impl GeometryBudget {
    /// Creates a fresh budget for one feature geometry.
    ///
    /// # Returns
    /// A geometry budget initialized to the parser's allocation limits.
    pub(super) fn new() -> Self {
        Self {
            coordinates: MAX_COORDINATES as usize,
            members: MAX_COORDINATES as usize,
        }
    }

    /// Reserves coordinate tuples against the aggregate geometry allocation cap.
    ///
    /// - `count`: Number of coordinate tuples requested by the current geometry node.
    ///
    /// # Returns
    /// Unit after the requested coordinate budget is consumed.
    ///
    /// # Errors
    /// Returns a diagnostic when nested geometry exceeds the total coordinate cap.
    fn take_coordinates(&mut self, count: u32) -> Result<(), String> {
        Self::take(&mut self.coordinates, count, "coordinate")
    }

    /// Reserves nested members against the aggregate geometry allocation cap.
    ///
    /// - `count`: Number of rings or child geometries requested.
    ///
    /// # Returns
    /// Unit after the requested member budget is consumed.
    ///
    /// # Errors
    /// Returns a diagnostic when nested geometry exceeds the total member cap.
    fn take_members(&mut self, count: u32) -> Result<(), String> {
        Self::take(&mut self.members, count, "member")
    }

    /// Decreases one remaining budget and rejects aggregate overflow.
    ///
    /// - `remaining`: Mutable remaining amount.
    /// - `count`: Requested amount.
    /// - `kind`: Safe budget label used in diagnostics.
    ///
    /// # Returns
    /// Unit after reservation.
    ///
    /// # Errors
    /// Returns an error when the request exceeds the remaining allowance.
    fn take(remaining: &mut usize, count: u32, kind: &str) -> Result<(), String> {
        let count = count as usize;
        if count > *remaining {
            return Err(format!("WKB aggregate {kind} limit is exceeded"));
        }
        *remaining -= count;
        Ok(())
    }
}

/// Resolves ISO WKB and EWKB geometry type flags.
///
/// - `raw`: Encoded 32-bit WKB type word.
///
/// # Returns
/// Base type, Z flag, M flag, and embedded SRID flag.
fn geometry_kind(raw: u32) -> (u32, bool, bool, bool) {
    let has_z = raw & 0x8000_0000 != 0;
    let has_m = raw & 0x4000_0000 != 0;
    let has_srid = raw & 0x2000_0000 != 0;
    let base = raw & 0x1fff_ffff;
    if has_z || has_m || has_srid {
        (base, has_z, has_m, has_srid)
    } else if (1000..2000).contains(&base) {
        (base - 1000, true, false, false)
    } else if (2000..3000).contains(&base) {
        (base - 2000, false, true, false)
    } else if (3000..4000).contains(&base) {
        (base - 3000, true, true, false)
    } else {
        (base, false, false, false)
    }
}

/// Reads one bounded count field and rejects pathological geometry allocations.
///
/// - `cursor`: WKB input cursor.
/// - `little`: WKB integer byte order.
///
/// # Returns
/// Validated geometry element count.
///
/// # Errors
/// Returns a diagnostic when the count exceeds the adapter limit.
fn read_count(cursor: &mut Cursor<&[u8]>, little: bool) -> Result<u32, String> {
    let count = read_u32(cursor, little)?;
    if count > MAX_COORDINATES {
        return Err("WKB coordinate/member count exceeds the supported limit".to_owned());
    }
    Ok(count)
}

/// Reads one coordinate and preserves each ordinate as round-trippable decimal text.
///
/// - `cursor`: WKB input cursor.
/// - `little`: WKB numeric byte order.
/// - `has_z`: Whether a third ordinate follows XY.
/// - `has_m`: Whether a measure ordinate follows XY/Z.
///
/// # Returns
/// Exact text values for the represented floating-point ordinates.
///
/// # Errors
/// Returns a diagnostic for truncation or non-finite source coordinates.
fn read_coordinate(
    cursor: &mut Cursor<&[u8]>,
    little: bool,
    has_z: bool,
    has_m: bool,
) -> Result<Coordinate, String> {
    let x = read_f64(cursor, little)?;
    let y = read_f64(cursor, little)?;
    let z = has_z.then(|| read_f64(cursor, little)).transpose()?;
    let measure = has_m.then(|| read_f64(cursor, little)).transpose()?;
    Ok(Coordinate {
        x: x.to_string(),
        y: y.to_string(),
        z: z.map(|value| value.to_string()),
        measure: measure.map(|value| value.to_string()),
    })
}

/// Reads one endian-aware unsigned integer.
///
/// - `cursor`: WKB input cursor.
/// - `little`: Whether integer bytes are little-endian.
///
/// # Returns
/// Decoded unsigned integer.
///
/// # Errors
/// Returns a diagnostic for truncated input.
fn read_u32(cursor: &mut Cursor<&[u8]>, little: bool) -> Result<u32, String> {
    let mut bytes = [0; 4];
    cursor
        .read_exact(&mut bytes)
        .map_err(|error| binary_error(&error))?;
    Ok(if little {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    })
}

/// Reads one finite endian-aware WKB double.
///
/// - `cursor`: WKB input cursor.
/// - `little`: Whether floating-point bytes are little-endian.
///
/// # Returns
/// Decoded finite coordinate ordinate.
///
/// # Errors
/// Returns a diagnostic for truncation or a non-finite ordinate.
fn read_f64(cursor: &mut Cursor<&[u8]>, little: bool) -> Result<f64, String> {
    let mut bytes = [0; 8];
    cursor
        .read_exact(&mut bytes)
        .map_err(|error| binary_error(&error))?;
    let value = if little {
        f64::from_le_bytes(bytes)
    } else {
        f64::from_be_bytes(bytes)
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("WKB coordinate is non-finite".to_owned())
    }
}

/// Converts binary read failures to stable geometry diagnostics.
///
/// - `error`: Input error raised by a bounded cursor read.
///
/// # Returns
/// Safe parser diagnostic.
fn binary_error(error: &std::io::Error) -> String {
    format!("truncated WKB geometry: {error}")
}
