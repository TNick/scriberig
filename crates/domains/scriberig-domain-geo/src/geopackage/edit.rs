//! Builds and reopens staged `GeoPackage` geometry replacements.

use rusqlite::{Connection, params};
use scriberig_core::EntityId;

mod attribute;
mod codec;

pub use attribute::{replace_feature_attribute_in_geopackage, validate_geopackage_feature_edit};

#[cfg(test)]
#[path = "edit/tests.rs"]
mod tests;

use crate::{
    Feature, GeoPackageError, GeoPackageReader, Geometry,
    replace_feature_geometry as replace_feature_successor,
};
use codec::{encode_gpkg_geometry, geometry_bounds, merge_bounds};

/// Replaces one feature geometry in staged `GeoPackage` bytes.
///
/// The source byte slice and any live file remain untouched. The writer refuses
/// packages with extension registrations or table triggers because their
/// invariants cannot be reproduced by this adapter. It preserves all SQLite
/// tables and unrelated feature rows, refreshes the target layer extent, and
/// reopens the output before returning it.
///
/// # Parameters
/// - `bytes`: Exact source `GeoPackage` bytes.
/// - `dataset`: Stable semantic dataset identity used to resolve the layer.
/// - `layer`: Stable semantic layer identity from that dataset.
/// - `native_id`: Exact provider row ID identifying the feature.
/// - `geometry`: Replacement geometry in the layer's CRS.
///
/// # Returns
/// Complete staged `GeoPackage` bytes with only the selected geometry row changed.
///
/// # Errors
/// Returns an error for malformed packages, missing features, incompatible
/// geometry, or package structures outside the supported safe-write subset.
pub fn replace_feature_geometry_in_geopackage(
    bytes: &[u8],
    dataset: EntityId,
    layer: EntityId,
    native_id: &str,
    geometry: &Geometry,
) -> Result<Vec<u8>, GeoPackageError> {
    let reader = GeoPackageReader::open(bytes, dataset)?;
    let info = reader
        .layers()
        .iter()
        .find(|info| info.layer.id == layer)
        .ok_or(GeoPackageError::UnknownLayer)?;
    if info.geometry_column.is_none() || info.srs_id.is_none() {
        return Err(GeoPackageError::UnsupportedEdit);
    }
    let (source_feature, extent) = scan_layer(&reader, layer, native_id)?;
    let source_feature = source_feature.ok_or(GeoPackageError::UnknownFeature)?;
    let successor = replace_feature_successor(&info.layer, &source_feature, geometry.clone())
        .map_err(|error| GeoPackageError::InvalidFeature(error.code.to_owned()))?;
    let srs_id = info.srs_id.ok_or(GeoPackageError::UnsupportedEdit)?;
    let geometry_column = info
        .geometry_column
        .as_deref()
        .ok_or(GeoPackageError::UnsupportedEdit)?;
    let encoded = encode_gpkg_geometry(srs_id, geometry)?;
    let connection = mutable_connection(bytes)?;
    reject_unpreserved_package_features(&connection, &info.table_name)?;
    let update = format!(
        "UPDATE {} SET {}=?1 WHERE {}=?2",
        super::quote_identifier(&info.table_name),
        super::quote_identifier(geometry_column),
        super::quote_identifier(&info.fid_column),
    );
    let changed = connection
        .execute(&update, params![encoded, native_id])
        .map_err(super::database_error)?;
    if changed != 1 {
        return Err(GeoPackageError::UnknownFeature);
    }
    refresh_extent(&connection, &info.table_name, extent, geometry)?;
    let output = connection
        .serialize(rusqlite::MAIN_DB)
        .map_err(super::database_error)?
        .to_vec();
    verify_staged(&output, dataset, layer, native_id, &successor)?;
    Ok(output)
}

/// Validates that staged `GeoPackage` bytes contain one geometry-only feature edit.
///
/// Every layer catalog and attribute schema must remain stable. Feature
/// identities, attributes, row counts, and layer membership must remain equal;
/// exactly one geometry value must differ across the whole dataset.
///
/// # Parameters
/// - `before`: Exact base package bytes.
/// - `after`: Exact staged package bytes.
///
/// # Returns
/// Unit when the staged package contains one semantic geometry change.
///
/// # Errors
/// Returns an error when either package is unreadable or changes exceed the
/// supported single-feature geometry operation.
pub fn validate_geopackage_geometry_edit(
    before: &[u8],
    after: &[u8],
) -> Result<(), GeoPackageError> {
    let dataset = EntityId::new();
    let before_reader = GeoPackageReader::open(before, dataset)?;
    let after_reader = GeoPackageReader::open(after, dataset)?;
    if before_reader.layers().len() != after_reader.layers().len() {
        return Err(GeoPackageError::UnsupportedEdit);
    }
    let mut changed_geometries = 0_usize;
    for (before_layer, after_layer) in before_reader.layers().iter().zip(after_reader.layers()) {
        if before_layer.layer != after_layer.layer
            || before_layer.table_name != after_layer.table_name
            || before_layer.fid_column != after_layer.fid_column
            || before_layer.geometry_column != after_layer.geometry_column
            || before_layer.srs_id != after_layer.srs_id
            || before_layer.feature_count != after_layer.feature_count
        {
            return Err(GeoPackageError::UnsupportedEdit);
        }
        changed_geometries +=
            compare_layer_features(&before_reader, &after_reader, before_layer.layer.id)?;
    }
    if changed_geometries != 1 {
        return Err(GeoPackageError::UnsupportedEdit);
    }
    Ok(())
}

/// Compares two exact-snapshot feature collections and counts geometry edits.
///
/// # Parameters
/// - `before`: Reader for the base package.
/// - `after`: Reader for the staged package.
/// - `layer`: Stable semantic layer identity present in both readers.
///
/// # Returns
/// Number of feature geometry changes.
///
/// # Errors
/// Returns an error when row identity, attributes, or layer membership differ.
fn compare_layer_features(
    before: &GeoPackageReader,
    after: &GeoPackageReader,
    layer: EntityId,
) -> Result<usize, GeoPackageError> {
    let schema = after
        .layers()
        .iter()
        .find(|info| info.layer.id == layer)
        .ok_or(GeoPackageError::UnknownLayer)?;
    let mut offset = 0;
    let mut changed = 0;
    loop {
        let left = before.feature_page(layer, offset, 1000)?;
        let right = after.feature_page(layer, offset, 1000)?;
        if left.has_more != right.has_more || left.features.len() != right.features.len() {
            return Err(GeoPackageError::UnsupportedEdit);
        }
        for (old, new) in left.features.iter().zip(&right.features) {
            if old.id != new.id
                || old.layer != new.layer
                || old.native_id != new.native_id
                || old.attributes != new.attributes
            {
                return Err(GeoPackageError::UnsupportedEdit);
            }
            if old.geometry != new.geometry {
                let geometry = new
                    .geometry
                    .clone()
                    .ok_or(GeoPackageError::UnsupportedEdit)?;
                let successor = replace_feature_successor(&schema.layer, old, geometry)
                    .map_err(|_| GeoPackageError::UnsupportedEdit)?;
                if successor != *new {
                    return Err(GeoPackageError::UnsupportedEdit);
                }
                changed += 1;
            }
        }
        if !left.has_more {
            break;
        }
        offset = offset
            .checked_add(1000)
            .ok_or(GeoPackageError::InvalidOffset)?;
    }
    Ok(changed)
}

/// Reads a selected feature and bounds the whole layer in pages.
///
/// # Parameters
/// - `reader`: Immutable reader over the exact source bytes.
/// - `layer`: Stable semantic layer identity.
/// - `native_id`: Feature's external primary key.
///
/// # Returns
/// Selected semantic feature and combined XY bounds, if any feature has geometry.
///
/// # Errors
/// Returns an error when source feature pages cannot be decoded.
fn scan_layer(
    reader: &GeoPackageReader,
    layer: EntityId,
    native_id: &str,
) -> Result<(Option<Feature>, Option<[f64; 4]>), GeoPackageError> {
    let mut offset = 0;
    let mut target = None;
    let mut bounds = None;
    loop {
        let page = reader.feature_page(layer, offset, 1000)?;
        for feature in page.features {
            if feature.native_id.as_deref() == Some(native_id) {
                target = Some(feature.clone());
            }
            if let Some(geometry) = feature.geometry.as_ref() {
                merge_bounds(&mut bounds, geometry_bounds(geometry)?);
            }
        }
        offset += 1000;
        if !page.has_more {
            break;
        }
    }
    Ok((target, bounds))
}

/// Opens exact source bytes as a mutable in-memory SQLite database.
///
/// # Parameters
/// - `bytes`: Exact serialized SQLite package.
///
/// # Returns
/// Mutable connection isolated from the source byte slice.
///
/// # Errors
/// Returns an error when SQLite cannot deserialize the source package.
fn mutable_connection(bytes: &[u8]) -> Result<Connection, GeoPackageError> {
    let mut connection = Connection::open_in_memory().map_err(super::database_error)?;
    connection
        .deserialize_read_exact(
            rusqlite::MAIN_DB,
            std::io::Cursor::new(bytes),
            bytes.len(),
            false,
        )
        .map_err(super::database_error)?;
    Ok(connection)
}

/// Rejects package features whose invariants are not maintained by this writer.
///
/// # Parameters
/// - `connection`: Mutable package connection.
/// - `table`: Exact selected feature table name.
///
/// # Returns
/// Unit when the selected table has no unsupported trigger or extension.
///
/// # Errors
/// Returns `UnsupportedEdit` when extensions or table triggers are registered.
fn reject_unpreserved_package_features(
    connection: &Connection,
    table: &str,
) -> Result<(), GeoPackageError> {
    let trigger_count: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='trigger' AND tbl_name=?1",
            params![table],
            |row| row.get(0),
        )
        .map_err(super::database_error)?;
    let extension_table = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='gpkg_extensions')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(super::database_error)?;
    let extension_count = if extension_table {
        connection
            .query_row(
                "SELECT count(*) FROM gpkg_extensions WHERE table_name=?1 OR table_name IS NULL",
                params![table],
                |row| row.get::<_, i64>(0),
            )
            .map_err(super::database_error)?
    } else {
        0
    };
    if trigger_count != 0 || extension_count != 0 {
        return Err(GeoPackageError::UnsupportedEdit);
    }
    Ok(())
}

/// Recomputes XY bounds from existing layer features plus the replacement.
///
/// # Parameters
/// - `connection`: Mutable package connection.
/// - `table`: Exact selected feature table.
/// - `previous_bounds`: Bounds computed from source geometries.
/// - `replacement`: New semantic geometry.
///
/// # Returns
/// Unit after the catalog extent is updated.
///
/// # Errors
/// Returns an error when bounds are invalid or catalog metadata cannot update.
fn refresh_extent(
    connection: &Connection,
    table: &str,
    mut previous_bounds: Option<[f64; 4]>,
    replacement: &Geometry,
) -> Result<(), GeoPackageError> {
    merge_bounds(&mut previous_bounds, geometry_bounds(replacement)?);
    let (min_x, min_y, max_x, max_y) = previous_bounds.map_or((None, None, None, None), |bounds| {
        (
            Some(bounds[0]),
            Some(bounds[1]),
            Some(bounds[2]),
            Some(bounds[3]),
        )
    });
    connection
        .execute(
            "UPDATE gpkg_contents SET min_x=?1, min_y=?2, max_x=?3, max_y=?4,
                 last_change=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE table_name=?5",
            params![min_x, min_y, max_x, max_y, table],
        )
        .map_err(super::database_error)?;
    Ok(())
}

/// Reopens staged bytes and verifies the selected semantic feature and schema.
///
/// # Parameters
/// - `bytes`: Complete staged `GeoPackage` bytes.
/// - `dataset`: Stable dataset identity.
/// - `layer`: Stable layer identity.
/// - `native_id`: Selected provider feature ID.
/// - `expected`: Expected feature successor.
///
/// # Returns
/// Unit when the selected feature and layer schema match the staged expectation.
///
/// # Errors
/// Returns a structured feature validation error for mismatch or reopen failure.
fn verify_staged(
    bytes: &[u8],
    dataset: EntityId,
    layer: EntityId,
    native_id: &str,
    expected: &Feature,
) -> Result<(), GeoPackageError> {
    let reader = GeoPackageReader::open(bytes, dataset)?;
    let info = reader
        .layers()
        .iter()
        .find(|info| info.layer.id == layer)
        .ok_or(GeoPackageError::UnknownLayer)?;
    let mut offset = 0;
    loop {
        let page = reader.feature_page(layer, offset, 1000)?;
        if let Some(actual) = page
            .features
            .iter()
            .find(|feature| feature.native_id.as_deref() == Some(native_id))
        {
            if actual == expected && info.srs_id.is_some() && info.geometry_column.is_some() {
                return Ok(());
            }
            return Err(GeoPackageError::InvalidFeature(
                "staged feature differs from the requested semantic successor".to_owned(),
            ));
        }
        if !page.has_more {
            return Err(GeoPackageError::UnknownFeature);
        }
        offset += 1000;
    }
}
