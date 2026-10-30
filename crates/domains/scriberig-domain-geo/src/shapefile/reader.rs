//! Reads bounded two-dimensional Shapefile feature pages with DBF attributes.

use std::collections::BTreeMap;

use scriberig_core::EntityId;

use crate::{Feature, FeaturePage, GeoDataset, GeoLayer, semantic_feature_id};

#[path = "format.rs"]
mod format;

/// Describes malformed, unsupported, or inconsistent Shapefile members.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapefileError {
    /// Shapefile header or record structure is malformed.
    InvalidShape,
    /// DBF header, schema, or record structure is malformed.
    InvalidAttributes,
    /// Shape type is outside the deterministic supported two-dimensional subset.
    UnsupportedShapeType,
    /// Required shape or attribute record counts do not agree.
    RecordCountMismatch,
    /// Requested feature page has an invalid size.
    InvalidPageSize,
}

/// Stores one bounded Shapefile layer and borrowed compound-source bytes.
///
/// # Public members
/// - `open`: Opens one exact sidecar group.
/// - `dataset`: Returns the stable dataset identity and layer membership.
/// - `layer`: Returns schema, CRS, extent/count summary, and fidelity notes.
/// - `feature_page`: Reads a bounded exact page of geometry and attributes.
///
/// # Private members
/// - `shape_bytes`: Borrowed exact `.shp` bytes.
/// - `attribute_bytes`: Borrowed exact `.dbf` bytes when available.
/// - `shape_records`: Validated record offsets and lengths.
/// - `dbf`: Parsed DBF header/schema metadata.
#[derive(Clone, Debug)]
pub struct ShapefileReader<'a> {
    /// Borrowed exact Shapefile geometry bytes.
    shape_bytes: &'a [u8],
    /// Borrowed exact dBase attribute bytes.
    attribute_bytes: Option<&'a [u8]>,
    /// Validated record offsets and content lengths.
    shape_records: Vec<(usize, usize)>,
    /// Parsed DBF schema and row layout.
    dbf: Option<format::DbfHeader>,
    /// Active source record indices, excluding dBase deleted rows.
    active_records: Vec<usize>,
    /// Provider-neutral dataset identity.
    dataset: GeoDataset,
    /// Provider-neutral single-layer semantics.
    layer: GeoLayer,
}

/// Returns an exact Shapefile dataset and its sole layer schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapefileLayer {
    /// Dataset and its ordered layer identity.
    pub dataset: GeoDataset,
    /// Stable semantic layer identity.
    pub layer: GeoLayer,
    /// Source row count without decoding feature payloads.
    pub feature_count: u64,
    /// Safe diagnostics describing the read-only provider subset.
    pub diagnostics: Vec<String>,
}

impl<'a> ShapefileReader<'a> {
    /// Opens one validated `.shp` and its optional `.dbf` and `.prj` members.
    ///
    /// Only two-dimensional Point, `PolyLine`, Polygon, `MultiPoint`, and Null
    /// records are accepted. This first provider remains explicitly read-only.
    ///
    /// # Parameters
    /// - `shape_bytes`: Exact `.shp` member bytes.
    /// - `attribute_bytes`: Exact `.dbf` member bytes, when present.
    /// - `projection_bytes`: Exact `.prj` member bytes, when present.
    /// - `dataset_id`: Stable semantic dataset identity.
    /// - `layer_name`: Authored filename stem used as the layer name.
    ///
    /// # Returns
    /// Reader with stable dataset/layer identities and validated member metadata.
    ///
    /// # Errors
    /// Returns a structured error for malformed, inconsistent, or unsupported data.
    pub fn open(
        shape_bytes: &'a [u8],
        attribute_bytes: Option<&'a [u8]>,
        projection_bytes: Option<&'a [u8]>,
        dataset_id: EntityId,
        layer_name: &str,
    ) -> Result<Self, ShapefileError> {
        let (shape_type, shape_records) = format::shape_records(shape_bytes)?;
        let dbf = attribute_bytes.map(format::dbf_header).transpose()?;
        if let Some(header) = &dbf
            && header.records != shape_records.len()
        {
            return Err(ShapefileError::RecordCountMismatch);
        }
        let active_records = match (&dbf, attribute_bytes) {
            (Some(header), Some(bytes)) => (0..shape_records.len())
                .filter_map(|index| match format::is_deleted(bytes, header, index) {
                    Ok(false) => Some(Ok(index)),
                    Ok(true) => None,
                    Err(error) => Some(Err(error)),
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => (0..shape_records.len()).collect(),
        };
        let layer_id = format::semantic_layer_id(dataset_id, layer_name);
        let crs = projection_bytes.map(format::projection_crs).transpose()?;
        let fields = dbf
            .as_ref()
            .map(|header| header.fields.clone())
            .unwrap_or_default();
        let geometry_type = format::geometry_type(shape_type)?;
        let layer = GeoLayer {
            id: layer_id,
            dataset: dataset_id,
            name: layer_name.to_owned(),
            fields,
            crs: crs.clone(),
            geometry_type,
        };
        Ok(Self {
            shape_bytes,
            attribute_bytes,
            shape_records,
            dbf,
            active_records,
            dataset: GeoDataset {
                id: dataset_id,
                layers: vec![layer_id],
            },
            layer,
        })
    }

    /// Returns dataset and layer metadata for the exact sidecar group.
    ///
    /// # Returns
    /// Stable dataset/layer identities, schema, count, and read-only diagnostics.
    #[must_use]
    pub fn layer(&self) -> ShapefileLayer {
        ShapefileLayer {
            dataset: self.dataset.clone(),
            layer: self.layer.clone(),
            feature_count: u64::try_from(self.active_records.len()).unwrap_or(u64::MAX),
            diagnostics: vec![
                "Shapefile semantic reads are supported; compound materialization is inspect-only"
                    .to_owned(),
            ],
        }
    }

    /// Reads one bounded page of features without exposing native record handles.
    ///
    /// # Parameters
    /// - `layer`: Exact semantic layer identity.
    /// - `offset`: Zero-based feature offset.
    /// - `limit`: Page size from one through one thousand.
    ///
    /// # Returns
    /// Features with stable Scriberig identities and source record numbers.
    ///
    /// # Errors
    /// Returns an error when the layer, page size, or source records are invalid.
    pub fn feature_page(
        &self,
        layer: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<FeaturePage, ShapefileError> {
        if layer != self.layer.id {
            return Err(ShapefileError::InvalidShape);
        }
        if limit == 0 || limit > 1_000 {
            return Err(ShapefileError::InvalidPageSize);
        }
        let start = offset.min(self.active_records.len());
        let end = start.saturating_add(limit).min(self.active_records.len());
        let mut features = Vec::with_capacity(end.saturating_sub(start));
        for active_index in start..end {
            let index = self.active_records[active_index];
            let (record_offset, record_length) = self.shape_records[index];
            let record = self
                .shape_bytes
                .get(record_offset..record_offset + record_length)
                .ok_or(ShapefileError::InvalidShape)?;
            let native_id = (index + 1).to_string();
            let geometry = format::geometry(record)?;
            let attributes = match (&self.attribute_bytes, &self.dbf) {
                (Some(bytes), Some(header)) => format::attributes(bytes, header, index)?,
                _ => BTreeMap::default(),
            };
            features.push(Feature {
                id: semantic_feature_id(self.dataset.id, self.layer.id, &native_id),
                layer: self.layer.id,
                native_id: Some(native_id),
                geometry,
                attributes,
            });
        }
        Ok(FeaturePage {
            features,
            has_more: end < self.active_records.len(),
        })
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
