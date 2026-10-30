//! Tests staged `GeoPackage` edits for semantic and package preservation.

use super::{
    replace_feature_attribute_in_geopackage, replace_feature_geometry_in_geopackage,
    validate_geopackage_feature_edit,
};
use crate::{GeoPackageReader, GeoValue, Geometry};
use rusqlite::Connection;
use scriberig_core::EntityId;

/// Creates a two-feature point `GeoPackage` for staged-writer tests.
///
/// # Returns
/// Serialized package with parcels 124 and 125.
fn fixture() -> Vec<u8> {
    let connection = Connection::open_in_memory().expect("open fixture database");
    connection
        .execute_batch(
            "CREATE TABLE gpkg_contents (
                table_name TEXT PRIMARY KEY, data_type TEXT NOT NULL,
                identifier TEXT, description TEXT DEFAULT '',
                last_change DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER
            );
            CREATE TABLE gpkg_geometry_columns (
                table_name TEXT, column_name TEXT, geometry_type_name TEXT,
                srs_id INTEGER, z TINYINT, m TINYINT
            );
            CREATE TABLE gpkg_spatial_ref_sys (
                srs_name TEXT, srs_id INTEGER PRIMARY KEY, organization TEXT,
                organization_coordsys_id INTEGER, definition TEXT, description TEXT
            );
            INSERT INTO gpkg_spatial_ref_sys VALUES
                ('WGS 84', 4326, 'EPSG', 4326, 'EPSG:4326', '');
            CREATE TABLE parcels (
                fid INTEGER PRIMARY KEY, geom BLOB, parcel_id INTEGER NOT NULL,
                area REAL, label TEXT
            );
            INSERT INTO gpkg_contents
                (table_name,data_type,identifier,min_x,min_y,max_x,max_y,srs_id)
                VALUES ('parcels','features','Parcels',1,2,3,4,4326);
            INSERT INTO gpkg_geometry_columns
                VALUES ('parcels','geom','POINT',4326,0,0);",
        )
        .expect("create package catalog");
    let point = point_blob(4326, 2.0, 3.0);
    connection
        .execute(
            "INSERT INTO parcels VALUES (124, ?1, 124, 10.5, 'Parcel 124'),
                (125, ?1, 125, 12.0, 'Parcel 125')",
            rusqlite::params![point],
        )
        .expect("insert features");
    connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize package")
        .to_vec()
}

/// Encodes one little-endian 2D `GeoPackage` point blob.
///
/// # Parameters
/// - `srs_id`: Spatial reference identifier for the package header.
/// - `x`: Point x ordinate.
/// - `y`: Point y ordinate.
///
/// # Returns
/// Encoded `GeoPackage` geometry blob.
fn point_blob(srs_id: i32, x: f64, y: f64) -> Vec<u8> {
    let mut bytes = b"GP\0\x01".to_vec();
    bytes.extend_from_slice(&srs_id.to_le_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&x.to_le_bytes());
    bytes.extend_from_slice(&y.to_le_bytes());
    bytes
}

/// Updates parcel 124 and preserves parcel 125 attributes and geometry.
#[test]
fn staged_geometry_edit_reopens_and_preserves_unrelated_feature() {
    let package = fixture();
    let dataset = EntityId::new();
    let reader = GeoPackageReader::open(&package, dataset).expect("source package");
    let layer = reader.layers()[0].layer.id;
    let before_other = reader.feature_page(layer, 1, 1).expect("neighbor").features[0].clone();
    let replacement = Geometry::Point(crate::Coordinate {
        x: "2.5".to_owned(),
        y: "3.5".to_owned(),
        z: None,
        measure: None,
    });

    let output =
        replace_feature_geometry_in_geopackage(&package, dataset, layer, "124", &replacement)
            .expect("staged `GeoPackage` edit");

    assert_ne!(output, package);
    let reopened = GeoPackageReader::open(&output, dataset).expect("reopen staged package");
    let page = reopened.feature_page(layer, 0, 2).expect("both features");
    assert_eq!(page.features[0].geometry, Some(replacement));
    assert_eq!(page.features[1], before_other);
    assert_eq!(
        reopened.layers()[0].layer.fields,
        reader.layers()[0].layer.fields
    );
    assert_eq!(reopened.layers()[0].srs_id, reader.layers()[0].srs_id);
    assert_eq!(reopened.layers()[0].extent.as_ref().unwrap()[0], "2");
}

/// Updates one typed parcel attribute and preserves neighboring feature semantics.
#[test]
fn staged_attribute_edit_checks_schema_and_preserves_unrelated_feature() {
    let package = fixture();
    let dataset = EntityId::new();
    let reader = GeoPackageReader::open(&package, dataset).expect("source package");
    let layer = reader.layers()[0].layer.id;
    let before_other = reader.feature_page(layer, 1, 1).expect("neighbor").features[0].clone();

    let output = replace_feature_attribute_in_geopackage(
        &package,
        dataset,
        layer,
        "124",
        "area",
        &GeoValue::Number("11.5".into()),
    )
    .expect("schema-valid staged attribute edit");

    validate_geopackage_feature_edit(&package, &output).expect("single typed attribute edit");
    let reopened = GeoPackageReader::open(&output, dataset).expect("reopen staged package");
    let page = reopened.feature_page(layer, 0, 2).expect("both features");
    assert_eq!(
        page.features[0].attributes.get("area"),
        Some(&GeoValue::Number("11.5".into()))
    );
    assert_eq!(page.features[1], before_other);
    assert_eq!(
        reopened.layers()[0].layer.fields,
        reader.layers()[0].layer.fields
    );
    assert_eq!(reopened.layers()[0].extent, reader.layers()[0].extent);
}

/// Rejects attribute values incompatible with schema and nullability.
#[test]
fn staged_attribute_edit_rejects_wrong_value_type_and_null() {
    let package = fixture();
    let dataset = EntityId::new();
    let reader = GeoPackageReader::open(&package, dataset).expect("source package");
    let layer = reader.layers()[0].layer.id;
    assert!(
        replace_feature_attribute_in_geopackage(
            &package,
            dataset,
            layer,
            "124",
            "area",
            &GeoValue::Text("large".into()),
        )
        .is_err()
    );
    assert!(
        replace_feature_attribute_in_geopackage(
            &package,
            dataset,
            layer,
            "124",
            "parcel_id",
            &GeoValue::Null,
        )
        .is_err()
    );
}

/// Rejects source package triggers that could mutate unrelated data on update.
#[test]
fn staged_geometry_edit_rejects_table_triggers() {
    let connection = super::mutable_connection(&fixture()).expect("mutable copy");
    connection
        .execute_batch("CREATE TRIGGER parcel_audit AFTER UPDATE ON parcels BEGIN SELECT 1; END;")
        .expect("create preservation-sensitive trigger");
    let package = connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize triggered package")
        .to_vec();
    let dataset = EntityId::new();
    let reader = GeoPackageReader::open(&package, dataset).expect("read triggered package");
    let layer = reader.layers()[0].layer.id;
    let geometry = Geometry::Point(crate::Coordinate {
        x: "2.5".to_owned(),
        y: "3.5".to_owned(),
        z: None,
        measure: None,
    });

    let error = replace_feature_geometry_in_geopackage(&package, dataset, layer, "124", &geometry)
        .expect_err("triggered layer must be read-only");

    assert_eq!(error, crate::GeoPackageError::UnsupportedEdit);
    assert!(!reader.layers()[0].editable);
}

/// Rejects layer extensions when their post-edit invariants are unknown.
#[test]
fn staged_geometry_edit_rejects_layer_extensions() {
    let connection = super::mutable_connection(&fixture()).expect("mutable copy");
    connection
        .execute_batch(
            "CREATE TABLE gpkg_extensions (
                table_name TEXT, column_name TEXT, extension_name TEXT,
                definition TEXT, scope TEXT
            );
            INSERT INTO gpkg_extensions VALUES
                ('parcels','geom','example_extension','urn:example','read-write');",
        )
        .expect("create extension metadata");
    let package = connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize extension package")
        .to_vec();
    let dataset = EntityId::new();
    let reader = GeoPackageReader::open(&package, dataset).expect("read extended package");
    let layer = reader.layers()[0].layer.id;
    let geometry = Geometry::Point(crate::Coordinate {
        x: "2.5".to_owned(),
        y: "3.5".to_owned(),
        z: None,
        measure: None,
    });

    let error = replace_feature_geometry_in_geopackage(&package, dataset, layer, "124", &geometry)
        .expect_err("layer extensions must be preserved by a future writer or block edits");

    assert_eq!(error, crate::GeoPackageError::UnsupportedEdit);
    assert!(!reader.layers()[0].editable);
}

/// Rejects a native feature ID that is absent from the exact source package.
#[test]
fn staged_geometry_edit_rejects_missing_feature() {
    let package = fixture();
    let dataset = EntityId::new();
    let reader = GeoPackageReader::open(&package, dataset).expect("source package");
    let layer = reader.layers()[0].layer.id;
    let geometry = Geometry::Point(crate::Coordinate {
        x: "2.5".to_owned(),
        y: "3.5".to_owned(),
        z: None,
        measure: None,
    });

    let error =
        replace_feature_geometry_in_geopackage(&package, dataset, layer, "missing", &geometry)
            .expect_err("unknown feature must be rejected");

    assert_eq!(error, crate::GeoPackageError::UnknownFeature);
    assert_eq!(reader.feature_page(layer, 0, 2).unwrap().features.len(), 2);
}
