//! Reads bounded `GeoPackage` table rows into provider-neutral feature values.

use std::collections::BTreeMap;

use rusqlite::{Connection, params, types::ValueRef};

use crate::{
    Feature, FeaturePage, FieldType, GeoDataset, GeoPackageError, GeoPackageLayerInfo, GeoValue,
    semantic_feature_id,
};

/// Maximum textual attribute and native-ID bytes returned by one page.
const MAX_PAGE_TEXT_BYTES: usize = 16 * 1024 * 1024;

/// Reads one deterministic feature page without materializing the full layer.
///
/// - `connection`: Read-only package connection.
/// - `dataset`: Stable semantic dataset owner.
/// - `layer`: Selected layer metadata.
/// - `offset`: Number of rows skipped.
/// - `limit`: Requested bounded page size.
///
/// Features with stable semantic IDs and native row IDs retained as evidence.
///
/// # Returns
/// Bounded feature page and continuation flag.
///
/// # Errors
/// Returns an error when source rows contain unsupported or malformed values.
pub(super) fn read_features(
    connection: &Connection,
    dataset: &GeoDataset,
    layer: &GeoPackageLayerInfo,
    offset: usize,
    limit: usize,
) -> Result<FeaturePage, GeoPackageError> {
    let sql = format!(
        "SELECT * FROM {} ORDER BY {} LIMIT ?1 OFFSET ?2",
        super::quote_identifier(&layer.table_name),
        super::quote_identifier(&layer.fid_column)
    );
    let mut statement = connection.prepare(&sql).map_err(super::database_error)?;
    let columns = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let geometry_index = layer
        .geometry_column
        .as_deref()
        .and_then(|name| columns.iter().position(|column| column == name));
    let fid_index = columns
        .iter()
        .position(|column| column == &layer.fid_column)
        .ok_or_else(|| GeoPackageError::InvalidPackage("feature ID column disappeared".into()))?;
    let query_limit = limit
        .checked_add(1)
        .ok_or(GeoPackageError::InvalidPageSize)?;
    let query_limit = i64::try_from(query_limit).map_err(|_| GeoPackageError::InvalidPageSize)?;
    let offset = i64::try_from(offset).map_err(|_| GeoPackageError::InvalidOffset)?;
    let mut rows = statement
        .query(params![query_limit, offset])
        .map_err(super::database_error)?;
    let mut features = Vec::with_capacity(limit);
    let mut geometry_budget = wkb::GeometryBudget::new();
    let mut text_budget = MAX_PAGE_TEXT_BYTES;
    let mut has_more = false;
    while let Some(row) = rows.next().map_err(super::database_error)? {
        if features.len() == limit {
            has_more = true;
            break;
        }
        let native_id = sqlite_scalar(
            row.get_ref(fid_index).map_err(super::database_error)?,
            &mut text_budget,
        )?;
        let id = semantic_feature_id(dataset.id, layer.layer.id, &native_id);
        let geometry = geometry_index
            .map(|index| {
                let value = row.get_ref(index).map_err(super::database_error)?;
                match value {
                    ValueRef::Null => Ok(None),
                    ValueRef::Blob(bytes) => {
                        wkb::parse_geopackage_geometry(bytes, layer.srs_id, &mut geometry_budget)
                            .map_err(GeoPackageError::InvalidFeature)
                    }
                    _ => Err(GeoPackageError::InvalidFeature(
                        "geometry column is not a GeoPackage blob".to_owned(),
                    )),
                }
            })
            .transpose()?
            .flatten();
        let mut attributes = BTreeMap::new();
        for (index, column) in columns.iter().enumerate() {
            if index == fid_index || Some(index) == geometry_index {
                continue;
            }
            let Some(field) = layer
                .layer
                .fields
                .iter()
                .find(|field| field.name == *column)
            else {
                continue;
            };
            let value = row.get_ref(index).map_err(super::database_error)?;
            if let Some(value) = semantic_value(value, field.field_type, &mut text_budget)? {
                attributes.insert(column.clone(), value);
            }
        }
        features.push(Feature {
            id,
            layer: layer.layer.id,
            native_id: Some(native_id),
            geometry,
            attributes,
        });
    }
    Ok(FeaturePage { features, has_more })
}

/// Converts one non-geometry SQLite value into the semantic attribute domain.
///
/// - `value`: SQLite source value.
/// - `field_type`: Schema type from `GeoPackage` metadata.
/// - `text_budget`: Remaining text allowance for the current feature page.
///
/// # Returns
/// Typed value, or `None` for binary data that remains only in exact source evidence.
///
/// # Errors
/// Returns an error when text cannot be decoded as UTF-8 or the page text budget is exceeded.
fn semantic_value(
    value: ValueRef<'_>,
    field_type: FieldType,
    text_budget: &mut usize,
) -> Result<Option<GeoValue>, GeoPackageError> {
    let semantic = match value {
        ValueRef::Null => GeoValue::Null,
        ValueRef::Integer(value) if matches!(field_type, FieldType::Boolean) => {
            GeoValue::Boolean(value != 0)
        }
        ValueRef::Integer(value) => GeoValue::Number(value.to_string()),
        ValueRef::Real(value) if value.is_finite() => GeoValue::Number(value.to_string()),
        ValueRef::Real(_) => {
            return Err(GeoPackageError::InvalidFeature(
                "non-finite real attribute is unsupported".to_owned(),
            ));
        }
        ValueRef::Text(bytes) => {
            consume_text_budget(text_budget, bytes.len())?;
            let value = std::str::from_utf8(bytes)
                .map_err(|_| GeoPackageError::InvalidFeature("attribute text is not UTF-8".into()))?
                .to_owned();
            if matches!(field_type, FieldType::DateTime) {
                GeoValue::DateTime(value)
            } else {
                GeoValue::Text(value)
            }
        }
        ValueRef::Blob(_) => return Ok(None),
    };
    Ok(Some(semantic))
}

/// Formats a source primary key as stable external identity evidence.
///
/// - `value`: SQLite primary-key value.
/// - `text_budget`: Remaining text allowance for the current feature page.
///
/// # Returns
/// Deterministic string representation of the native ID.
///
/// # Errors
/// Returns an error for a binary, malformed, or non-finite primary key.
fn sqlite_scalar(value: ValueRef<'_>, text_budget: &mut usize) -> Result<String, GeoPackageError> {
    match value {
        ValueRef::Integer(value) => Ok(value.to_string()),
        ValueRef::Real(value) if value.is_finite() => Ok(value.to_string()),
        ValueRef::Text(bytes) => {
            consume_text_budget(text_budget, bytes.len())?;
            std::str::from_utf8(bytes)
                .map(str::to_owned)
                .map_err(|_| GeoPackageError::InvalidFeature("feature ID is not UTF-8".into()))
        }
        ValueRef::Null | ValueRef::Blob(_) | ValueRef::Real(_) => Err(
            GeoPackageError::InvalidFeature("feature ID is not a supported scalar".to_owned()),
        ),
    }
}

/// Consumes the bounded page allowance for source text bytes.
///
/// - `remaining`: Remaining text capacity for this page.
/// - `requested`: Source text byte length.
///
/// Unit after the requested bytes are accounted for.
///
/// # Returns
/// Unit when the requested bytes fit within the budget.
///
/// # Errors
/// Returns an error when page text exceeds the configured limit.
fn consume_text_budget(remaining: &mut usize, requested: usize) -> Result<(), GeoPackageError> {
    if requested > *remaining {
        return Err(GeoPackageError::InvalidFeature(
            "feature page text exceeds the 16 MiB response limit".to_owned(),
        ));
    }
    *remaining -= requested;
    Ok(())
}

mod wkb;

#[cfg(test)]
mod tests {
    use super::consume_text_budget;
    use crate::GeoPackageError;

    /// Rejects page text that would exceed its total memory budget.
    #[test]
    fn oversized_feature_page_text_is_rejected() {
        let mut remaining = 2;
        assert_eq!(consume_text_budget(&mut remaining, 2), Ok(()));
        assert_eq!(remaining, 0);
        assert_eq!(
            consume_text_budget(&mut remaining, 1),
            Err(GeoPackageError::InvalidFeature(
                "feature page text exceeds the 16 MiB response limit".to_owned()
            ))
        );
    }
}
