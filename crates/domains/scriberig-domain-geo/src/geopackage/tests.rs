//! Verifies bounded `GeoPackage` metadata and feature access.

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use scriberig_core::EntityId;

    use crate::{GeoPackageError, GeoPackageReader};
    use crate::{GeoValue, Geometry};

    /// Creates a minimal in-memory `GeoPackage` with two parcel point features.
    ///
    /// # Parameters
    /// - `geometry_srs`: SRS ID encoded in source feature geometry.
    ///
    /// # Returns
    /// Serialized package bytes with layer metadata and two point records.
    fn fixture(geometry_srs: i32) -> Vec<u8> {
        let connection = Connection::open_in_memory().expect("open fixture db");
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
            .expect("create GeoPackage catalog");
        let geometry = point_blob(geometry_srs, 2.0, 3.0);
        connection
            .execute(
                "INSERT INTO parcels VALUES (124, ?1, 124, 10.5, 'Parcel 124'),
                    (125, ?1, 125, 12.0, 'Parcel 125')",
                rusqlite::params![geometry],
            )
            .expect("insert fixture features");
        connection
            .serialize(rusqlite::MAIN_DB)
            .expect("serialize package")
            .to_vec()
    }

    /// Encodes one little-endian `GeoPackage` point geometry.
    ///
    /// # Parameters
    /// - `srs_id`: SRS identifier in the package geometry header.
    /// - `x`: Point x ordinate.
    /// - `y`: Point y ordinate.
    ///
    /// # Returns
    /// Encoded two-dimensional point geometry.
    fn point_blob(srs_id: i32, x: f64, y: f64) -> Vec<u8> {
        let mut bytes = b"GP\0\x01".to_vec();
        bytes.extend_from_slice(&srs_id.to_le_bytes());
        bytes.push(1);
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&x.to_le_bytes());
        bytes.extend_from_slice(&y.to_le_bytes());
        bytes
    }

    /// Imports metadata first and reads only the requested bounded feature page.
    #[test]
    fn reads_layer_metadata_and_bounded_semantic_features() {
        let package = fixture(4326);
        let dataset_id = EntityId::new();
        let reader = GeoPackageReader::open(&package, dataset_id).expect("open GeoPackage");
        assert_eq!(reader.dataset().layers.len(), 1);
        let layer = &reader.layers()[0];
        assert_eq!(layer.layer.name, "parcels");
        assert_eq!(layer.feature_count, 2);
        assert_eq!(layer.layer.fields.len(), 3);
        assert!(layer.editable);
        assert_eq!(layer.diagnostics, [] as [String; 0]);
        assert_eq!(
            layer.layer.crs.as_ref().and_then(|crs| crs.code.as_deref()),
            Some("4326")
        );
        assert_eq!(
            layer.extent.as_ref().map(|extent| extent[0].as_str()),
            Some("1")
        );

        let page = reader
            .feature_page(layer.layer.id, 0, 1)
            .expect("first feature page");
        assert_eq!(page.features.len(), 1);
        assert!(page.has_more);
        let feature = &page.features[0];
        assert_eq!(feature.native_id.as_deref(), Some("124"));
        assert_eq!(
            feature.attributes.get("parcel_id"),
            Some(&GeoValue::Number("124".into()))
        );
        assert_eq!(
            feature.attributes.get("area"),
            Some(&GeoValue::Number("10.5".into()))
        );
        assert_eq!(
            feature.geometry,
            Some(Geometry::Point(crate::Coordinate {
                x: "2".into(),
                y: "3".into(),
                z: None,
                measure: None,
            }))
        );
        assert_eq!(
            reader.feature_page(layer.layer.id, 0, 0),
            Err(GeoPackageError::InvalidPageSize)
        );
    }

    /// Rejects ordinary SQLite databases without the `GeoPackage` catalog.
    #[test]
    fn rejects_sqlite_without_geopackage_catalog() {
        let connection = Connection::open_in_memory().expect("open fixture db");
        let bytes = connection
            .serialize(rusqlite::MAIN_DB)
            .expect("serialize sqlite db")
            .to_vec();
        assert_eq!(
            GeoPackageReader::open(&bytes, EntityId::new()).err(),
            Some(GeoPackageError::NotGeoPackage)
        );
    }

    /// Rejects geometry whose embedded CRS conflicts with the layer metadata.
    #[test]
    fn rejects_geometry_with_mismatched_spatial_reference_id() {
        let package = fixture(3857);
        let reader = GeoPackageReader::open(&package, EntityId::new()).expect("open package");
        let layer = reader.layers()[0].layer.id;
        assert_eq!(
            reader.feature_page(layer, 0, 1),
            Err(GeoPackageError::InvalidFeature(
                "GeoPackage geometry CRS differs from its layer metadata".to_owned()
            ))
        );
    }
}
