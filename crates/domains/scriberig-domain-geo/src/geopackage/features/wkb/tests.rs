//! Verifies bounded `GeoPackage` WKB decoding across supported geometry families.

use crate::Geometry;

use super::{GeometryBudget, MAX_COORDINATES, parse_geopackage_geometry};

/// Wraps WKB bytes with a little-endian `GeoPackage` geometry header.
///
/// - `wkb`: Standard WKB payload.
/// - `srs_id`: `GeoPackage` spatial reference identifier.
///
/// `GeoPackage` geometry blob.
/// # Returns
/// A geometry blob containing the header, spatial reference, and WKB payload.
fn geopackage_blob(wkb: &[u8], srs_id: i32) -> Vec<u8> {
    let mut bytes = b"GP\0\x01".to_vec();
    bytes.extend_from_slice(&srs_id.to_le_bytes());
    bytes.extend_from_slice(wkb);
    bytes
}

/// Appends one little-endian WKB point to an output buffer.
///
/// - `bytes`: WKB output buffer.
/// - `x`: Point x ordinate.
/// - `y`: Point y ordinate.
fn append_point(bytes: &mut Vec<u8>, x: f64, y: f64) {
    bytes.push(1);
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&x.to_le_bytes());
    bytes.extend_from_slice(&y.to_le_bytes());
}

/// Decodes a two-vertex line string as semantic coordinates.
#[test]
fn reads_linestring_geometry() {
    let mut wkb = vec![1];
    wkb.extend_from_slice(&2_u32.to_le_bytes());
    wkb.extend_from_slice(&2_u32.to_le_bytes());
    for (x, y) in [(1.25_f64, -2.5_f64), (3.0, 4.0)] {
        wkb.extend_from_slice(&x.to_le_bytes());
        wkb.extend_from_slice(&y.to_le_bytes());
    }
    let mut budget = GeometryBudget::new();
    assert_eq!(
        parse_geopackage_geometry(&geopackage_blob(&wkb, 4326), Some(4326), &mut budget),
        Ok(Some(Geometry::LineString(vec![
            coordinate("1.25", "-2.5"),
            coordinate("3", "4"),
        ])))
    );
}

/// Decodes one polygon shell and retains its ordered source vertices.
#[test]
fn reads_polygon_rings() {
    let mut wkb = vec![1];
    wkb.extend_from_slice(&3_u32.to_le_bytes());
    wkb.extend_from_slice(&1_u32.to_le_bytes());
    wkb.extend_from_slice(&4_u32.to_le_bytes());
    for (x, y) in [(0.0_f64, 0.0_f64), (5.0, 0.0), (5.0, 5.0), (0.0, 0.0)] {
        wkb.extend_from_slice(&x.to_le_bytes());
        wkb.extend_from_slice(&y.to_le_bytes());
    }
    let mut budget = GeometryBudget::new();
    let geometry = parse_geopackage_geometry(&geopackage_blob(&wkb, 4326), Some(4326), &mut budget)
        .expect("decode polygon");
    assert_eq!(
        geometry,
        Some(Geometry::Polygon(vec![vec![
            coordinate("0", "0"),
            coordinate("5", "0"),
            coordinate("5", "5"),
            coordinate("0", "0"),
        ]]))
    );
}

/// Decodes nested collection members while sharing the page geometry budget.
#[test]
fn reads_geometry_collection() {
    let mut wkb = vec![1];
    wkb.extend_from_slice(&7_u32.to_le_bytes());
    wkb.extend_from_slice(&1_u32.to_le_bytes());
    append_point(&mut wkb, 8.5, 9.5);
    let mut budget = GeometryBudget::new();
    assert_eq!(
        parse_geopackage_geometry(&geopackage_blob(&wkb, 4326), Some(4326), &mut budget),
        Ok(Some(Geometry::Collection(vec![Geometry::Point(
            coordinate("8.5", "9.5",)
        )])))
    );
}

/// Rejects aggregate geometry allocations above the configured limit.
#[test]
fn rejects_geometry_member_counts_above_the_limit() {
    let mut wkb = vec![1];
    wkb.extend_from_slice(&7_u32.to_le_bytes());
    wkb.extend_from_slice(&(MAX_COORDINATES + 1).to_le_bytes());
    let mut budget = GeometryBudget::new();
    assert!(
        parse_geopackage_geometry(&geopackage_blob(&wkb, 4326), Some(4326), &mut budget)
            .expect_err("oversized collection is rejected")
            .contains("limit")
    );
}

/// Creates one exact semantic coordinate for expected geometry values.
///
/// - `x`: Expected x decimal text.
/// - `y`: Expected y decimal text.
///
/// Two-dimensional coordinate with no measure.
/// # Returns
/// A coordinate containing the exact authored ordinates.
fn coordinate(x: &str, y: &str) -> crate::Coordinate {
    crate::Coordinate {
        x: x.to_owned(),
        y: y.to_owned(),
        z: None,
        measure: None,
    }
}
