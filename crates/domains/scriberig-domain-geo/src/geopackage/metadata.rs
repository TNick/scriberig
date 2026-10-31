//! Loads `GeoPackage` catalog, schema, CRS, and stable layer identities.

use rusqlite::{Connection, OptionalExtension, params};
use scriberig_core::EntityId;

use super::{GeoPackageError, GeoPackageLayerInfo};
use crate::{CoordinateReferenceSystem, Field, FieldType, GeoLayer, GeometryType};

/// Loads vector table catalog entries without reading their feature rows.
///
/// - `connection`: Read-only package connection.
/// - `dataset_id`: Stable owner for deterministic semantic layer identities.
///
/// # Returns
/// Ordered layer metadata read from the package catalog.
///
/// # Errors
/// Returns an error when required metadata is malformed or SQLite access fails.
pub(crate) fn load_layers(
    connection: &Connection,
    dataset_id: EntityId,
) -> Result<Vec<GeoPackageLayerInfo>, GeoPackageError> {
    let mut statement = connection
        .prepare(
            "SELECT table_name, srs_id, min_x, min_y, max_x, max_y
             FROM gpkg_contents WHERE data_type='features' ORDER BY table_name",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i32>>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<f64>>(5)?,
            ))
        })
        .map_err(database_error)?;
    let mut layers = Vec::new();
    for row in rows {
        let (table, contents_srs, min_x, min_y, max_x, max_y) = row.map_err(database_error)?;
        if let Some(layer) = load_layer(
            connection,
            dataset_id,
            &table,
            contents_srs,
            [min_x, min_y, max_x, max_y],
        )? {
            layers.push(layer);
        }
    }
    Ok(layers)
}

/// Loads schema and geometry metadata for one vector table.
///
/// - `connection`: Read-only package connection.
/// - `dataset_id`: Stable semantic dataset identity.
/// - `table`: Exact table name from the trusted `GeoPackage` catalog.
/// - `contents_srs`: Optional CRS ID from `gpkg_contents`.
/// - `extent`: Optional authored extent coordinates.
///
/// # Returns
/// Semantic layer summary when the catalog table exists, otherwise `None`.
///
/// # Errors
/// Returns an error for malformed table metadata or SQLite access failure.
fn load_layer(
    connection: &Connection,
    dataset_id: EntityId,
    table: &str,
    contents_srs: Option<i32>,
    extent: [Option<f64>; 4],
) -> Result<Option<GeoPackageLayerInfo>, GeoPackageError> {
    let mut geometry = connection
        .prepare(
            "SELECT column_name, geometry_type_name, srs_id
             FROM gpkg_geometry_columns WHERE table_name=?1",
        )
        .map_err(database_error)?;
    let geometry_row = geometry
        .query_row(params![table], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })
        .optional()
        .map_err(database_error)?;
    let columns = table_columns(connection, table)?;
    let geometry_column = geometry_row.as_ref().map(|metadata| metadata.0.as_str());
    let Some((fid_column, fields)) = feature_schema(columns, geometry_column) else {
        return Err(GeoPackageError::InvalidPackage(
            "feature table has no stable primary key".to_owned(),
        ));
    };
    let mut diagnostics = Vec::new();
    if geometry_row
        .as_ref()
        .is_some_and(|(_, kind, _)| parse_geometry_type(kind).is_none())
    {
        diagnostics.push("layer geometry type is not represented by the semantic schema".into());
    }
    let table_id = layer_id(dataset_id, table);
    let (geometry_column, geometry_type, crs_id) = match geometry_row {
        Some((column, kind, srs)) => (Some(column), parse_geometry_type(&kind), Some(srs)),
        None => (None, None, contents_srs),
    };
    let editable = layer_write_capability(connection, table, geometry_type.is_some(), crs_id)?;
    if !editable {
        diagnostics
            .push("layer has package structures outside the safe geometry writer subset".into());
    }
    let crs = crs_id.map(|id| load_crs(connection, id)).transpose()?;
    let count_sql = format!("SELECT count(*) FROM {}", quote_identifier(table));
    let feature_count = connection
        .query_row(&count_sql, [], |row| row.get::<_, i64>(0))
        .map_err(database_error)?;
    let feature_count = u64::try_from(feature_count).map_err(|_| {
        GeoPackageError::InvalidPackage("feature count is negative or out of range".to_owned())
    })?;
    Ok(Some(GeoPackageLayerInfo {
        layer: GeoLayer {
            id: table_id,
            dataset: dataset_id,
            name: table.to_owned(),
            fields,
            crs,
            geometry_type,
        },
        table_name: table.to_owned(),
        fid_column,
        srs_id: crs_id,
        geometry_column,
        feature_count,
        extent: extent_strings(extent),
        editable,
        diagnostics,
    }))
}

/// Checks whether this layer has structures the staged geometry writer supports.
///
/// # Parameters
/// - `connection`: Exact package connection.
/// - `table`: Feature table name.
/// - `supported_geometry`: Whether the layer geometry kind is modeled.
/// - `srs_id`: Layer spatial reference identifier.
///
/// # Returns
/// `true` when supported geometry metadata and preservation boundaries hold.
///
/// # Errors
/// Returns an error when package catalog queries fail.
fn layer_write_capability(
    connection: &Connection,
    table: &str,
    supported_geometry: bool,
    srs_id: Option<i32>,
) -> Result<bool, GeoPackageError> {
    if !supported_geometry || srs_id.is_none() {
        return Ok(false);
    }
    let triggers: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='trigger' AND tbl_name=?1",
            params![table],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let has_extensions = table_exists(connection, "gpkg_extensions")?;
    let extensions = if has_extensions {
        connection
            .query_row(
                "SELECT count(*) FROM gpkg_extensions WHERE table_name=?1 OR table_name IS NULL",
                params![table],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?
    } else {
        0
    };
    Ok(triggers == 0 && extensions == 0)
}

/// Returns true when the named catalog table exists.
///
/// - `connection`: Read-only package connection.
/// - `name`: SQLite table name to find.
///
/// # Returns
/// `true` when the named catalog table exists.
///
/// # Errors
/// Returns an error on SQLite access failure.
pub(super) fn table_exists(connection: &Connection, name: &str) -> Result<bool, GeoPackageError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            params![name],
            |row| row.get(0),
        )
        .map_err(database_error)
}

/// Loads ordered column names, declarations, and constraints from SQLite metadata.
///
/// - `connection`: Read-only package connection.
/// - `table`: Catalog table name.
///
/// # Returns
/// Table columns as `(name, declared type, primary-key order, nullable)` tuples.
///
/// # Errors
/// Returns an error on SQLite metadata failure.
fn table_columns(
    connection: &Connection,
    table: &str,
) -> Result<Vec<(String, String, i64, bool)>, GeoPackageError> {
    let sql = format!("PRAGMA table_info({})", quote_identifier(table));
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(3)? == 0,
            ))
        })
        .map_err(database_error)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)
}

/// Converts SQLite column metadata into the semantic feature schema.
///
/// - `columns`: Table metadata in SQLite column order.
/// - `geometry_column`: Geometry column excluded from ordinary fields.
///
/// # Returns
/// Primary-key column and ordered non-key semantic fields, or `None` without a primary key.
fn feature_schema(
    columns: Vec<(String, String, i64, bool)>,
    geometry_column: Option<&str>,
) -> Option<(String, Vec<Field>)> {
    let primary = columns.iter().find(|column| column.2 > 0)?.0.clone();
    let fields = columns
        .into_iter()
        .filter(|column| column.0 != primary && Some(column.0.as_str()) != geometry_column)
        .map(|(name, declaration, _, nullable)| Field {
            name,
            field_type: field_type(&declaration),
            nullable,
        })
        .collect();
    Some((primary, fields))
}

/// Converts a complete optional extent tuple to exact decimal text.
///
/// - `extent`: Optional authored minimum and maximum ordinates.
///
/// # Returns
/// Four-value exact text extent, or `None` when any bound is absent.
fn extent_strings(extent: [Option<f64>; 4]) -> Option<[String; 4]> {
    Some([
        extent[0]?.to_string(),
        extent[1]?.to_string(),
        extent[2]?.to_string(),
        extent[3]?.to_string(),
    ])
}

/// Maps a SQLite declared type to a provider-neutral scalar type.
///
/// - `declaration`: Authored SQLite type declaration.
///
/// # Returns
/// Conservative semantic field type.
fn field_type(declaration: &str) -> FieldType {
    let declaration = declaration.to_ascii_uppercase();
    if declaration.contains("INT")
        || declaration.contains("REAL")
        || declaration.contains("NUM")
        || declaration.contains("DEC")
        || declaration.contains("FLOA")
        || declaration.contains("DOUB")
    {
        FieldType::Number
    } else if declaration.contains("DATE") || declaration.contains("TIME") {
        FieldType::DateTime
    } else if declaration.contains("BLOB") {
        FieldType::Binary
    } else {
        FieldType::Text
    }
}

/// Parses a standard `GeoPackage` geometry type name when supported by the domain.
///
/// - `name`: Type name from `gpkg_geometry_columns`.
///
/// # Returns
/// Provider-neutral geometry kind, or `None` for unsupported/mixed declarations.
fn parse_geometry_type(name: &str) -> Option<GeometryType> {
    match name.to_ascii_uppercase().as_str() {
        "POINT" => Some(GeometryType::Point),
        "LINESTRING" => Some(GeometryType::LineString),
        "POLYGON" => Some(GeometryType::Polygon),
        "MULTIPOINT" | "MULTILINESTRING" | "MULTIPOLYGON" | "GEOMETRY" | "GEOMETRYCOLLECTION" => {
            Some(GeometryType::GeometryCollection)
        }
        _ => None,
    }
}

/// Reads CRS authority and exact source definition for one SRS identifier.
///
/// - `connection`: Read-only package connection.
/// - `srs_id`: Package coordinate-system identifier.
///
/// # Returns
/// Explicit coordinate reference system.
///
/// # Errors
/// Returns an error when the package has no corresponding CRS record.
fn load_crs(
    connection: &Connection,
    srs_id: i32,
) -> Result<CoordinateReferenceSystem, GeoPackageError> {
    connection
        .query_row(
            "SELECT organization, organization_coordsys_id, definition
             FROM gpkg_spatial_ref_sys WHERE srs_id=?1",
            params![srs_id],
            |row| {
                let authority = row.get::<_, String>(0)?;
                let code = row.get::<_, i32>(1)?;
                let definition = row.get::<_, String>(2)?;
                Ok(CoordinateReferenceSystem {
                    authority: Some(authority),
                    code: Some(code.to_string()),
                    definition: Some(definition),
                })
            },
        )
        .map_err(database_error)
}

/// Quotes one catalog identifier for dynamic table access.
///
/// - `identifier`: Exact identifier returned by a `GeoPackage` catalog table.
///
/// # Returns
/// SQLite quoted identifier safe for interpolation.
pub(crate) fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Derives a stable semantic layer identity from dataset and table key.
///
/// - `dataset`: Stable owning dataset identity.
/// - `table`: Adapter-defined table name from the persisted package.
///
/// # Returns
/// Deterministic Scriberig layer identity.
fn layer_id(dataset: EntityId, table: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-geopackage-layer-v1\0");
    hasher.update(dataset.as_bytes());
    hasher.update(table.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Converts an SQLite failure to safe package-reader diagnostic text.
///
/// - `error`: SQLite operation failure.
///
/// # Returns
/// Structured adapter error without exposing native SQLite objects.
pub(crate) fn database_error(error: rusqlite::Error) -> GeoPackageError {
    match error {
        rusqlite::Error::SqliteFailure(code, message) => {
            GeoPackageError::InvalidPackage(message.map_or_else(
                || format!("SQLite failure: {code:?}"),
                |text| format!("SQLite failure: {code:?}: {text}"),
            ))
        }
        other => GeoPackageError::InvalidPackage(format!("SQLite operation failed: {other:?}")),
    }
}
