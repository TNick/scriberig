//! Stages schema-checked `GeoPackage` feature attribute edits.

use rusqlite::{params, types::Value};
use scriberig_core::EntityId;

use crate::{
    Feature, GeoPackageError, GeoPackageReader, GeoValue,
    set_feature_attribute as feature_successor,
};

use super::{mutable_connection, reject_unpreserved_package_features, scan_layer, verify_staged};

/// Updates one typed feature attribute in complete staged `GeoPackage` bytes.
///
/// The source bytes remain untouched. Field type and nullability are checked
/// against the exact layer schema before the SQLite copy is changed. The writer
/// refuses unsupported triggers and extensions, then reopens and verifies the
/// staged result before returning it.
///
/// # Parameters
/// - `bytes`: Exact source `GeoPackage` bytes.
/// - `dataset`: Stable semantic dataset identity used to resolve the layer.
/// - `layer`: Stable semantic layer identity from that dataset.
/// - `native_id`: Exact provider row ID identifying the feature.
/// - `field`: Authored layer field to update.
/// - `value`: Typed replacement attribute value.
///
/// # Returns
/// Complete staged bytes with only the selected feature attribute changed.
///
/// # Errors
/// Returns an error for missing targets, schema violations, malformed packages,
/// or source features outside the supported safe-write subset.
pub fn replace_feature_attribute_in_geopackage(
    bytes: &[u8],
    dataset: EntityId,
    layer: EntityId,
    native_id: &str,
    field: &str,
    value: &GeoValue,
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
    let schema_field = info
        .layer
        .fields
        .iter()
        .find(|candidate| candidate.name == field)
        .ok_or(GeoPackageError::UnsupportedEdit)?;
    let (source_feature, _) = scan_layer(&reader, layer, native_id)?;
    let source_feature = source_feature.ok_or(GeoPackageError::UnknownFeature)?;
    let successor = feature_successor(&info.layer, &source_feature, field, value.clone())
        .map_err(|error| GeoPackageError::InvalidFeature(error.code.to_owned()))?;
    let connection = mutable_connection(bytes)?;
    reject_unpreserved_package_features(&connection, &info.table_name)?;
    let update = format!(
        "UPDATE {} SET {}=?1 WHERE {}=?2",
        super::super::quote_identifier(&info.table_name),
        super::super::quote_identifier(&schema_field.name),
        super::super::quote_identifier(&info.fid_column),
    );
    let changed = connection
        .execute(&update, params![sqlite_value(value), native_id])
        .map_err(super::super::database_error)?;
    if changed != 1 {
        return Err(GeoPackageError::UnknownFeature);
    }
    connection
        .execute(
            "UPDATE gpkg_contents SET last_change=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE table_name=?1",
            params![info.table_name],
        )
        .map_err(super::super::database_error)?;
    let output = connection
        .serialize(rusqlite::MAIN_DB)
        .map_err(super::super::database_error)?
        .to_vec();
    verify_staged(&output, dataset, layer, native_id, &successor)?;
    Ok(output)
}

/// Validates that staged bytes contain one schema-valid feature attribute or geometry change.
///
/// Layer schema, row identity, membership, and counts must remain stable. Exactly
/// one feature may change; its change must be a single geometry replacement or
/// one typed attribute update.
///
/// # Parameters
/// - `before`: Exact base package bytes.
/// - `after`: Exact staged package bytes.
///
/// # Returns
/// Unit when the packages differ by one supported feature change.
///
/// # Errors
/// Returns an error when packages cannot be read, schema changes, or feature
/// differences do not represent one valid operation.
pub fn validate_geopackage_feature_edit(
    before: &[u8],
    after: &[u8],
) -> Result<(), GeoPackageError> {
    let dataset = EntityId::new();
    let before_reader = GeoPackageReader::open(before, dataset)?;
    let after_reader = GeoPackageReader::open(after, dataset)?;
    if before_reader.layers().len() != after_reader.layers().len() {
        return Err(GeoPackageError::UnsupportedEdit);
    }
    let mut changed_features = 0;
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
        changed_features += compare_features(&before_reader, &after_reader, &before_layer.layer)?;
    }
    if changed_features != 1 {
        return Err(GeoPackageError::UnsupportedEdit);
    }
    Ok(())
}

/// Counts exact feature successors that satisfy the one-field or geometry operation contract.
///
/// # Parameters
/// - `before`: Reader for the base package.
/// - `after`: Reader for the staged package.
/// - `layer`: Exact layer schema being compared.
///
/// # Returns
/// Number of valid changed feature records.
///
/// # Errors
/// Returns an error when identities or feature values change outside the
/// supported single-field or geometry operation.
fn compare_features(
    before: &GeoPackageReader,
    after: &GeoPackageReader,
    layer: &crate::GeoLayer,
) -> Result<usize, GeoPackageError> {
    let mut offset = 0;
    let mut changed_features = 0;
    loop {
        let left = before.feature_page(layer.id, offset, 1000)?;
        let right = after.feature_page(layer.id, offset, 1000)?;
        if left.has_more != right.has_more || left.features.len() != right.features.len() {
            return Err(GeoPackageError::UnsupportedEdit);
        }
        for (old, new) in left.features.iter().zip(&right.features) {
            if old.id != new.id || old.layer != new.layer || old.native_id != new.native_id {
                return Err(GeoPackageError::UnsupportedEdit);
            }
            if old == new {
                continue;
            }
            let successor = if old.geometry != new.geometry && old.attributes == new.attributes {
                let geometry = new
                    .geometry
                    .clone()
                    .ok_or(GeoPackageError::UnsupportedEdit)?;
                crate::replace_feature_geometry(layer, old, geometry)
                    .map_err(|_| GeoPackageError::UnsupportedEdit)?
            } else if old.geometry == new.geometry {
                attribute_successor_from_maps(layer, old, new)?
            } else {
                return Err(GeoPackageError::UnsupportedEdit);
            };
            if successor != *new {
                return Err(GeoPackageError::UnsupportedEdit);
            }
            changed_features += 1;
        }
        if !left.has_more {
            break;
        }
        offset = offset
            .checked_add(1000)
            .ok_or(GeoPackageError::InvalidOffset)?;
    }
    Ok(changed_features)
}

/// Resolves a single differing field from the base and staged attribute maps.
///
/// # Parameters
/// - `layer`: Exact layer schema.
/// - `old`: Base feature value.
/// - `new`: Staged feature value.
///
/// # Returns
/// Schema-validated feature successor.
///
/// # Errors
/// Returns `UnsupportedEdit` for added/removed fields or multiple changed values.
fn attribute_successor_from_maps(
    layer: &crate::GeoLayer,
    old: &Feature,
    new: &Feature,
) -> Result<Feature, GeoPackageError> {
    if old.attributes.len() != new.attributes.len() {
        return Err(GeoPackageError::UnsupportedEdit);
    }
    let changed = old
        .attributes
        .iter()
        .filter(|(name, value)| new.attributes.get(*name) != Some(*value))
        .collect::<Vec<_>>();
    if changed.len() != 1 {
        return Err(GeoPackageError::UnsupportedEdit);
    }
    let (field, _) = changed[0];
    let value = new
        .attributes
        .get(field)
        .cloned()
        .ok_or(GeoPackageError::UnsupportedEdit)?;
    feature_successor(layer, old, field, value).map_err(|_| GeoPackageError::UnsupportedEdit)
}

/// Converts one schema-validated semantic value into a SQLite update value.
///
/// # Parameters
/// - `value`: Typed semantic feature attribute.
///
/// # Returns
/// SQLite scalar preserving null, text, numeric text, boolean, and date/time data.
fn sqlite_value(value: &GeoValue) -> Value {
    match value {
        GeoValue::Null => Value::Null,
        GeoValue::Text(value) | GeoValue::Number(value) | GeoValue::DateTime(value) => {
            Value::Text(value.clone())
        }
        GeoValue::Boolean(value) => Value::Integer(i64::from(*value)),
    }
}
