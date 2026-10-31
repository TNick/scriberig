//! Creates the deterministic `GeoPackage` source used by query integration tests.

use rusqlite::Connection;

/// Builds a two-feature point `GeoPackage` with exact parcel attribute values.
///
/// # Returns
/// Serialized SQLite package bytes with parcels 124 and 125.
pub(crate) fn package() -> Vec<u8> {
    let connection = Connection::open_in_memory().expect("fixture database");
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
                ('WGS 84',4326,'EPSG',4326,'EPSG:4326','');
            CREATE TABLE parcels (
                fid INTEGER PRIMARY KEY, geom BLOB, parcel_id INTEGER NOT NULL, name TEXT
            );
            INSERT INTO gpkg_contents
                (table_name,data_type,identifier,min_x,min_y,max_x,max_y,srs_id)
                VALUES ('parcels','features','Parcels',1,2,30,40,4326);
            INSERT INTO gpkg_geometry_columns
                VALUES ('parcels','geom','POINT',4326,0,0);",
        )
        .expect("package catalog");
    let point_124 = point_blob(2.0, 3.0);
    let point_125 = point_blob(20.0, 30.0);
    connection
        .execute(
            "INSERT INTO parcels VALUES (124, ?1, 124, 'North'),
                (125, ?2, 125, 'South')",
            rusqlite::params![point_124, point_125],
        )
        .expect("parcel rows");
    connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize package")
        .to_vec()
}

/// Encodes a `GeoPackage` point with an exact WGS 84 header.
///
/// # Parameters
/// - `x`: Point easting.
/// - `y`: Point northing.
///
/// # Returns
/// `GeoPackage` geometry blob for one little-endian point.
fn point_blob(x: f64, y: f64) -> Vec<u8> {
    let mut bytes = b"GP\0\x01".to_vec();
    bytes.extend_from_slice(&4326_i32.to_le_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&x.to_le_bytes());
    bytes.extend_from_slice(&y.to_le_bytes());
    bytes
}
