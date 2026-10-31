//! Reads `GeoPackage` metadata and bounded vector feature pages.

mod edit;
mod features;

use std::io::Cursor;

use rusqlite::Connection;
use scriberig_core::EntityId;
use serde::{Deserialize, Serialize};

use crate::{FeaturePage, GeoDataset, GeoLayer};

/// Maximum accepted `GeoPackage` size for the in-memory read-only adapter.
const MAX_GEOPACKAGE_BYTES: usize = 512 * 1024 * 1024;

/// Describes a readable `GeoPackage` vector layer and its bounded read metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GeoPackageLayerInfo {
    /// Provider-neutral semantic layer schema.
    pub layer: GeoLayer,
    /// Exact `GeoPackage` table name used only for adapter reads.
    pub table_name: String,
    /// Primary key column used as external feature identity evidence.
    pub fid_column: String,
    /// Authored spatial reference ID used by `GeoPackage` geometry headers.
    pub srs_id: Option<i32>,
    /// Geometry column name, absent for attribute-only feature tables.
    pub geometry_column: Option<String>,
    /// Feature count reported by the table without loading feature rows.
    pub feature_count: u64,
    /// Authored XY extent when all four extent values are available.
    pub extent: Option<[String; 4]>,
    /// `GeoPackage` writer capability; false until staged package preservation exists.
    pub editable: bool,
    /// Safe explanation of unsupported layer/package behavior.
    pub diagnostics: Vec<String>,
}

/// Reads one immutable `GeoPackage` byte source without exposing SQLite types.
pub struct GeoPackageReader {
    /// Read-only in-memory SQLite database.
    connection: Connection,
    /// Stable dataset semantic identity assigned by the caller.
    dataset: GeoDataset,
    /// Ordered vector-layer metadata from the package catalog.
    layers: Vec<GeoPackageLayerInfo>,
}

pub use edit::{
    replace_feature_attribute_in_geopackage, replace_feature_geometry_in_geopackage,
    validate_geopackage_feature_edit, validate_geopackage_geometry_edit,
};

impl GeoPackageReader {
    /// Opens exact `GeoPackage` bytes as a bounded, read-only dataset view.
    ///
    /// - `bytes`: Exact SQLite package bytes.
    /// - `dataset_id`: Caller-owned semantic identity for the logical artifact.
    ///
    /// Read-only package metadata and stable layer identities.
    ///
    /// Returns an error for invalid/oversized packages or unsupported catalog data.
    /// # Returns
    /// Read-only dataset reader with stable layer identifiers.
    ///
    /// # Errors
    /// Returns an error when the source is oversized, malformed, or has unsupported catalog data.
    pub fn open(bytes: &[u8], dataset_id: EntityId) -> Result<Self, GeoPackageError> {
        if bytes.len() > MAX_GEOPACKAGE_BYTES {
            return Err(GeoPackageError::PackageTooLarge);
        }
        if !bytes.starts_with(b"SQLite format 3\0") {
            return Err(GeoPackageError::NotGeoPackage);
        }
        let mut connection = Connection::open_in_memory().map_err(database_error)?;
        connection
            .deserialize_read_exact(rusqlite::MAIN_DB, Cursor::new(bytes), bytes.len(), true)
            .map_err(database_error)?;
        if !metadata::table_exists(&connection, "gpkg_contents")? {
            return Err(GeoPackageError::NotGeoPackage);
        }
        let layers = load_layers(&connection, dataset_id)?;
        let dataset = GeoDataset {
            id: dataset_id,
            layers: layers.iter().map(|info| info.layer.id).collect(),
        };
        Ok(Self {
            connection,
            dataset,
            layers,
        })
    }

    /// Returns dataset identity and ordered layer references without feature rows.
    /// # Returns
    /// Stable dataset identity and ordered layer identifiers.
    #[must_use]
    pub fn dataset(&self) -> &GeoDataset {
        &self.dataset
    }

    /// Returns ordered layer schema and summary metadata.
    /// # Returns
    /// Ordered layer metadata loaded from the package catalog.
    #[must_use]
    pub fn layers(&self) -> &[GeoPackageLayerInfo] {
        &self.layers
    }

    /// Reads one bounded page of semantic features from the selected layer.
    ///
    /// - `layer_id`: Exact semantic layer identity.
    /// - `offset`: Number of source feature rows skipped.
    /// - `limit`: Requested page size, from one through one thousand.
    ///
    /// Bounded stable feature page with Scriberig-owned IDs and native IDs retained separately.
    ///
    /// Returns an error for an unknown layer, invalid page size, or source read failure.
    /// # Returns
    /// Bounded semantic features and continuation metadata.
    ///
    /// # Errors
    /// Returns an error for an unknown layer, invalid page size, or source read failure.
    pub fn feature_page(
        &self,
        layer_id: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<FeaturePage, GeoPackageError> {
        let layer = self
            .layers
            .iter()
            .find(|layer| layer.layer.id == layer_id)
            .ok_or(GeoPackageError::UnknownLayer)?;
        if !(1..=1000).contains(&limit) {
            return Err(GeoPackageError::InvalidPageSize);
        }
        self::features::read_features(&self.connection, &self.dataset, layer, offset, limit)
    }
}

/// Describes safe `GeoPackage` reader failures without leaking SQLite internals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GeoPackageError {
    /// Source does not have a valid `GeoPackage` catalog.
    NotGeoPackage,
    /// Source exceeds the bounded in-memory package limit.
    PackageTooLarge,
    /// A package catalog or database operation failed.
    InvalidPackage(String),
    /// The selected semantic layer is not present in this package.
    UnknownLayer,
    /// The selected native feature ID is not present in its layer.
    UnknownFeature,
    /// A feature page request exceeds the documented range.
    InvalidPageSize,
    /// The source row offset cannot be represented by the SQLite integer range.
    InvalidOffset,
    /// Geometry encoding is not supported by this read adapter.
    UnsupportedGeometry,
    /// Feature geometry or attributes contain malformed source data.
    InvalidFeature(String),
    /// A source package contains structures that the staged writer cannot preserve safely.
    UnsupportedEdit,
}

mod metadata;
pub(crate) use metadata::{database_error, load_layers, quote_identifier};

#[cfg(test)]
#[path = "tests.rs"]
mod geopackage_tests;
