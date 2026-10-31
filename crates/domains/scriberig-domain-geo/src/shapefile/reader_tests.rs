//! Verifies bounded Shapefile and DBF semantic reads.

use super::{ShapefileError, ShapefileReader};
use crate::{GeoValue, Geometry};
use scriberig_core::EntityId;

/// Builds a valid point Shapefile with one record per coordinate pair.
///
/// # Parameters
/// - `points`: Ordered coordinates to encode.
///
/// # Returns
/// Exact `.shp` bytes with valid big-endian record framing.
fn points_file(points: &[(f64, f64)]) -> Vec<u8> {
    let content_length = 20_usize;
    let total = 100 + points.len() * (8 + content_length);
    let mut bytes = vec![0; 100];
    bytes[0..4].copy_from_slice(&9994_i32.to_be_bytes());
    bytes[24..28].copy_from_slice(&i32::try_from(total / 2).expect("word length").to_be_bytes());
    bytes[28..32].copy_from_slice(&1000_i32.to_le_bytes());
    bytes[32..36].copy_from_slice(&1_i32.to_le_bytes());
    for (index, (x, y)) in points.iter().enumerate() {
        bytes.extend_from_slice(
            &i32::try_from(index + 1)
                .expect("record number")
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&10_i32.to_be_bytes());
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        bytes.extend_from_slice(&x.to_le_bytes());
        bytes.extend_from_slice(&y.to_le_bytes());
    }
    bytes
}

/// Builds a dBase file containing one numeric `parcel_id` field.
///
/// # Parameters
/// - `values`: Ordered field values matching the Shapefile records.
///
/// # Returns
/// Exact `.dbf` bytes with fixed-width rows.
fn parcel_attributes(values: &[&str]) -> Vec<u8> {
    let header_length = 65_u16;
    let record_length = 6_u16;
    let mut bytes = vec![0; usize::from(header_length)];
    bytes[0] = 3;
    bytes[4..8].copy_from_slice(
        &u32::try_from(values.len())
            .expect("record count")
            .to_le_bytes(),
    );
    bytes[8..10].copy_from_slice(&header_length.to_le_bytes());
    bytes[10..12].copy_from_slice(&record_length.to_le_bytes());
    bytes[32..43].copy_from_slice(b"parcel_id\0\0");
    bytes[43] = b'N';
    bytes[48] = 5;
    bytes[64] = 0x0d;
    for value in values {
        bytes.push(b' ');
        bytes.extend_from_slice(format!("{value:>5}").as_bytes());
    }
    bytes
}

/// Imports stable features and typed attributes from a real compound byte set.
#[test]
fn reads_bounded_shapefile_features_with_read_only_fidelity() {
    let dataset = EntityId::from_bytes([1; 16]);
    let shape = points_file(&[(1.25, 2.5), (8.0, 9.0)]);
    let attributes = parcel_attributes(&["124", "125"]);
    let projection = br#"GEOGCS["WGS 84"]"#;
    let reader = ShapefileReader::open(
        &shape,
        Some(&attributes),
        Some(projection),
        dataset,
        "parcels",
    )
    .expect("open Shapefile sidecar set");
    let summary = reader.layer();
    assert_eq!(summary.dataset.id, dataset);
    assert_eq!(summary.feature_count, 2);
    assert_eq!(summary.layer.fields[0].name, "parcel_id");
    assert_eq!(
        summary
            .layer
            .crs
            .as_ref()
            .and_then(|crs| crs.definition.as_deref()),
        Some("GEOGCS[\"WGS 84\"]")
    );
    assert!(summary.diagnostics[0].contains("inspect-only"));

    let first = reader
        .feature_page(summary.layer.id, 0, 1)
        .expect("first feature page");
    assert_eq!(first.features.len(), 1);
    assert!(first.has_more);
    assert_eq!(first.features[0].native_id.as_deref(), Some("1"));
    assert_eq!(
        first.features[0].attributes["parcel_id"],
        GeoValue::Number("124".to_owned())
    );
    assert_eq!(
        first.features[0].geometry,
        Some(Geometry::Point(crate::Coordinate {
            x: "1.25".to_owned(),
            y: "2.5".to_owned(),
            z: None,
            measure: None,
        }))
    );
    let second = reader
        .feature_page(summary.layer.id, 1, 1)
        .expect("second feature page");
    assert!(!second.has_more);
    assert_ne!(first.features[0].id, second.features[0].id);
}

/// Rejects a source whose attribute record count differs from its geometry rows.
#[test]
fn rejects_shapefile_dbf_row_count_mismatch() {
    let shape = points_file(&[(1.0, 2.0)]);
    let attributes = parcel_attributes(&["124", "125"]);
    assert_eq!(
        ShapefileReader::open(&shape, Some(&attributes), None, EntityId::new(), "parcels")
            .unwrap_err(),
        ShapefileError::RecordCountMismatch
    );
}

/// Excludes deleted DBF rows while retaining original source record identity.
#[test]
fn skips_deleted_dbf_rows_without_reindexing_source_features() {
    let shape = points_file(&[(1.0, 2.0), (8.0, 9.0)]);
    let mut attributes = parcel_attributes(&["124", "125"]);
    attributes[65] = b'*';
    let reader = ShapefileReader::open(&shape, Some(&attributes), None, EntityId::new(), "parcels")
        .expect("open Shapefile sidecar set");
    let summary = reader.layer();
    assert_eq!(summary.feature_count, 1);
    let page = reader
        .feature_page(summary.layer.id, 0, 10)
        .expect("active feature page");
    assert_eq!(page.features.len(), 1);
    assert_eq!(page.features[0].native_id.as_deref(), Some("2"));
    assert_eq!(
        page.features[0].attributes["parcel_id"],
        GeoValue::Number("125".to_owned())
    );
}

/// Rejects DBF memo columns whose external values are not in the sidecar set.
#[test]
fn rejects_dbf_memo_field_without_memo_sidecar_support() {
    let shape = points_file(&[(1.0, 2.0)]);
    let mut attributes = parcel_attributes(&["124"]);
    attributes[43] = b'M';
    assert_eq!(
        ShapefileReader::open(&shape, Some(&attributes), None, EntityId::new(), "parcels")
            .unwrap_err(),
        ShapefileError::InvalidAttributes
    );
}
