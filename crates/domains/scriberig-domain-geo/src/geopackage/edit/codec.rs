//! Encodes typed feature geometry as bounded `GeoPackage` and WKB values.

use crate::{GeoPackageError, Geometry};

/// Encodes a two-, three-, or four-dimensional geometry in a `GeoPackage` blob.
///
/// # Parameters
/// - `srs_id`: Layer spatial reference identifier.
/// - `geometry`: Typed geometry to encode.
///
/// # Returns
/// Little-endian `GeoPackage` header followed by ISO WKB.
///
/// # Errors
/// Returns an error for empty, mixed-dimensional, non-finite, or unsupported geometry.
pub(super) fn encode_gpkg_geometry(
    srs_id: i32,
    geometry: &Geometry,
) -> Result<Vec<u8>, GeoPackageError> {
    let dimensions = geometry_dimensions(geometry)?;
    let mut bytes = b"GP\0\x01".to_vec();
    bytes.extend_from_slice(&srs_id.to_le_bytes());
    encode_wkb(geometry, dimensions, &mut bytes)?;
    Ok(bytes)
}

/// Encodes typed geometry as little-endian ISO WKB.
///
/// # Parameters
/// - `geometry`: Geometry value to encode.
/// - `dimensions`: Shared coordinate dimensionality.
/// - `output`: Destination byte vector.
///
/// # Returns
/// Unit after one complete geometry is appended.
///
/// # Errors
/// Returns an error for empty or malformed geometry content.
fn encode_wkb(
    geometry: &Geometry,
    dimensions: u8,
    output: &mut Vec<u8>,
) -> Result<(), GeoPackageError> {
    output.push(1);
    let base = match geometry {
        Geometry::Point(_) => 1,
        Geometry::LineString(_) => 2,
        Geometry::Polygon(_) => 3,
        Geometry::Collection(_) => 7,
    };
    let iso = base
        + match dimensions {
            0 => 0,
            1 => 1000,
            2 => 2000,
            3 => 3000,
            _ => return Err(GeoPackageError::UnsupportedGeometry),
        };
    output.extend_from_slice(&iso_u32(iso));
    match geometry {
        Geometry::Point(point) => encode_coordinate(point, dimensions, output)?,
        Geometry::LineString(points) => {
            write_count(points.len(), output)?;
            for point in points {
                encode_coordinate(point, dimensions, output)?;
            }
        }
        Geometry::Polygon(rings) => {
            write_count(rings.len(), output)?;
            for ring in rings {
                write_count(ring.len(), output)?;
                for point in ring {
                    encode_coordinate(point, dimensions, output)?;
                }
            }
        }
        Geometry::Collection(items) => {
            write_count(items.len(), output)?;
            for item in items {
                encode_wkb(item, dimensions, output)?;
            }
        }
    }
    Ok(())
}

/// Writes one coordinate tuple as finite little-endian ordinates.
///
/// # Parameters
/// - `coordinate`: Exact decimal coordinate tuple.
/// - `dimensions`: Required tuple dimensionality.
/// - `output`: Destination WKB bytes.
///
/// # Returns
/// Unit after the tuple is appended.
///
/// # Errors
/// Returns an error for invalid numeric values or dimension mismatch.
fn encode_coordinate(
    coordinate: &crate::Coordinate,
    dimensions: u8,
    output: &mut Vec<u8>,
) -> Result<(), GeoPackageError> {
    let expected_dimensions =
        u8::from(coordinate.z.is_some()) + 2 * u8::from(coordinate.measure.is_some());
    if expected_dimensions != dimensions {
        return Err(GeoPackageError::UnsupportedGeometry);
    }
    let mut ordinates = vec![coordinate.x.as_str(), coordinate.y.as_str()];
    if dimensions & 1 != 0 {
        ordinates.push(
            coordinate
                .z
                .as_deref()
                .ok_or(GeoPackageError::UnsupportedGeometry)?,
        );
    }
    if dimensions & 2 != 0 {
        ordinates.push(
            coordinate
                .measure
                .as_deref()
                .ok_or(GeoPackageError::UnsupportedGeometry)?,
        );
    }
    for value in ordinates {
        let number = value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .ok_or(GeoPackageError::UnsupportedGeometry)?;
        output.extend_from_slice(&number.to_le_bytes());
    }
    Ok(())
}

/// Determines one nonempty and homogeneous coordinate dimensionality.
///
/// # Parameters
/// - `geometry`: Geometry whose coordinates are inspected.
///
/// # Returns
/// Shared XY, XYZ, or XYZM dimension count.
///
/// # Errors
/// Returns an error for empty or mixed-dimensional geometries.
fn geometry_dimensions(geometry: &Geometry) -> Result<u8, GeoPackageError> {
    let mut dimension = None;
    for coordinate in geometry_coordinates(geometry) {
        let current = u8::from(coordinate.z.is_some()) + 2 * u8::from(coordinate.measure.is_some());
        if dimension.is_some_and(|previous| previous != current) {
            return Err(GeoPackageError::UnsupportedGeometry);
        }
        dimension = Some(current);
    }
    dimension.ok_or(GeoPackageError::UnsupportedGeometry)
}

/// Traverses every coordinate in geometry order.
///
/// # Parameters
/// - `geometry`: Geometry value to traverse.
///
/// # Returns
/// Iterator over all coordinate tuples.
fn geometry_coordinates(geometry: &Geometry) -> Box<dyn Iterator<Item = &crate::Coordinate> + '_> {
    match geometry {
        Geometry::Point(point) => Box::new(std::iter::once(point)),
        Geometry::LineString(points) => Box::new(points.iter()),
        Geometry::Polygon(rings) => Box::new(rings.iter().flatten()),
        Geometry::Collection(items) => Box::new(items.iter().flat_map(geometry_coordinates)),
    }
}

/// Computes finite XY bounds for one semantic geometry.
///
/// # Parameters
/// - `geometry`: Geometry to measure.
///
/// # Returns
/// Minimum x/y and maximum x/y values.
///
/// # Errors
/// Returns an error when any x/y ordinate is not finite numeric text.
pub(super) fn geometry_bounds(geometry: &Geometry) -> Result<Option<[f64; 4]>, GeoPackageError> {
    let mut bounds: Option<[f64; 4]> = None;
    for coordinate in geometry_coordinates(geometry) {
        let x = finite(&coordinate.x)?;
        let y = finite(&coordinate.y)?;
        merge_bounds(&mut bounds, Some([x, y, x, y]));
    }
    Ok(bounds)
}

/// Merges one optional XY extent into a mutable accumulated extent.
///
/// # Parameters
/// - `current`: Extent accumulator.
/// - `next`: Extent to merge.
pub(super) fn merge_bounds(current: &mut Option<[f64; 4]>, next: Option<[f64; 4]>) {
    let Some(next) = next else { return };
    if let Some(current) = current {
        current[0] = current[0].min(next[0]);
        current[1] = current[1].min(next[1]);
        current[2] = current[2].max(next[2]);
        current[3] = current[3].max(next[3]);
    } else {
        *current = Some(next);
    }
}

/// Parses a finite exact-text ordinate for extent calculations.
///
/// # Parameters
/// - `value`: Decimal ordinate text.
///
/// # Returns
/// Finite floating-point value used only for package extent metadata.
///
/// # Errors
/// Returns an error for invalid or non-finite values.
fn finite(value: &str) -> Result<f64, GeoPackageError> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or(GeoPackageError::UnsupportedGeometry)
}

/// Encodes a WKB type code in little-endian byte order.
///
/// # Parameters
/// - `value`: Unsigned WKB code.
///
/// # Returns
/// Four little-endian bytes.
fn iso_u32(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Writes a bounded WKB member or coordinate count.
///
/// # Parameters
/// - `count`: Count to encode.
/// - `output`: Destination bytes.
///
/// # Returns
/// Unit after encoding the count.
///
/// # Errors
/// Returns an error when the count cannot fit the supported WKB range.
fn write_count(count: usize, output: &mut Vec<u8>) -> Result<(), GeoPackageError> {
    let count = u32::try_from(count).map_err(|_| GeoPackageError::UnsupportedGeometry)?;
    if count > 100_000 {
        return Err(GeoPackageError::UnsupportedGeometry);
    }
    output.extend_from_slice(&count.to_le_bytes());
    Ok(())
}
