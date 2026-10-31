//! Exposes bounded `GeoPackage` feature pages from the persisted project scan.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_geo::{FeaturePage, GeoDataset, GeoLayer, GeoPackageReader};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{ProjectError, ProjectService, support::adapters::geopackage::dataset_semantic_id};

/// Maximum semantic layer summaries returned in one `GeoPackage` catalog page.
pub const MAX_GEOPACKAGE_LAYER_PAGE: usize = 200;

/// Summarizes a `GeoPackage` and its ordered layer catalog at one exact snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoPackageCatalog {
    /// Exact semantic snapshot containing the source revision.
    pub snapshot: SnapshotId,
    /// Stable logical dataset identity.
    pub dataset: GeoDataset,
    /// Exact source member digest.
    pub source_digest: [u8; 32],
    /// Ordered bounded layer metadata, without enumerating feature rows.
    pub layers: Vec<GeoPackageLayerSummary>,
    /// Total catalog layer count before pagination.
    pub total: usize,
    /// Whether a later catalog page exists.
    pub has_more: bool,
}

/// Describes one semantic `GeoPackage` layer and its bounded catalog metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoPackageLayerSummary {
    /// Stable semantic layer identity and schema.
    pub layer: GeoLayer,
    /// Number of source features reported by the catalog.
    pub feature_count: u64,
    /// Authored layer extent in exact coordinate text, when available.
    pub extent: Option<[String; 4]>,
    /// Whether the current adapter can safely edit the layer.
    pub editable: bool,
    /// Safe diagnostics for unsupported or restricted behavior.
    pub diagnostics: Vec<String>,
}

impl ProjectService {
    /// Reads exact `GeoPackage` catalog metadata without loading feature rows.
    ///
    /// # Parameters
    /// - `project`: Project owning the persisted source scan.
    /// - `path`: Exact project-relative path from that scan.
    /// - `snapshot`: Immutable snapshot whose source revision is requested.
    /// - `offset`: Zero-based layer offset.
    /// - `limit`: Maximum layer summaries, at most two hundred.
    ///
    /// # Returns
    /// Dataset identity, source digest, ordered layer schema, and exact snapshot.
    ///
    /// # Errors
    /// Returns a project, storage, package, or snapshot error.
    pub fn geopackage_catalog(
        &self,
        project: ProjectId,
        path: &str,
        snapshot: SnapshotId,
        offset: usize,
        limit: usize,
    ) -> Result<GeoPackageCatalog, ProjectError> {
        if !(1..=MAX_GEOPACKAGE_LAYER_PAGE).contains(&limit) {
            return Err(geo_error(
                "invalid_page_size",
                "GeoPackage layer page limit must be between one and two hundred",
            ));
        }
        let file = self
            .store
            .project_files(project)
            .map_err(crate::errors::storage_debug_error)?
            .into_iter()
            .find(|file| file.relative_path == path && file.adapter == "geopackage")
            .ok_or_else(|| geo_error("artifact_not_found", "GeoPackage source was not found"))?;
        let revision = self
            .store
            .resolve(snapshot, file.entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                geo_error(
                    "source_not_in_snapshot",
                    "GeoPackage source is absent from the requested snapshot",
                )
            })?;
        let source = self
            .store
            .get(revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                geo_error(
                    "source_revision_missing",
                    "GeoPackage source revision is unavailable",
                )
            })?;
        let dataset_id = dataset_semantic_id(file.entity);
        let reader = GeoPackageReader::open(source.payload(), dataset_id).map_err(|_| {
            geo_error(
                "geopackage_invalid",
                "persisted GeoPackage could not be read",
            )
        })?;
        let layers = reader
            .layers()
            .iter()
            .map(|info| GeoPackageLayerSummary {
                layer: info.layer.clone(),
                feature_count: info.feature_count,
                extent: info.extent.clone(),
                editable: info.editable,
                diagnostics: info.diagnostics.clone(),
            })
            .collect::<Vec<_>>();
        if offset > layers.len() {
            return Err(geo_error(
                "offset_out_of_range",
                "GeoPackage layer offset exceeds the catalog size",
            ));
        }
        let total = layers.len();
        let page = layers
            .into_iter()
            .skip(offset)
            .take(limit + 1)
            .collect::<Vec<_>>();
        let has_more = page.len() > limit;
        Ok(GeoPackageCatalog {
            snapshot,
            dataset: reader.dataset().clone(),
            source_digest: *blake3::hash(source.payload()).as_bytes(),
            layers: page.into_iter().take(limit).collect(),
            total,
            has_more,
        })
    }

    /// Reads a bounded feature page from one exact persisted `GeoPackage` member.
    ///
    /// # Parameters
    /// - `project`: Project owning the persisted scan.
    /// - `path`: Exact project-relative path captured by the scan.
    /// - `snapshot`: Exact immutable source snapshot.
    /// - `layer`: Stable semantic layer identity from artifact inspection.
    /// - `offset`: Zero-based feature offset.
    /// - `limit`: Maximum returned feature count, at most one thousand.
    ///
    /// # Returns
    /// Bounded page of provider-neutral features.
    ///
    /// # Errors
    /// Returns a project, storage, package, or page validation error.
    pub fn geopackage_feature_page(
        &self,
        project: ProjectId,
        path: &str,
        snapshot: SnapshotId,
        layer: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<FeaturePage, ProjectError> {
        let file = self
            .store
            .project_files(project)
            .map_err(crate::errors::storage_debug_error)?
            .into_iter()
            .find(|file| file.relative_path == path && file.adapter == "geopackage")
            .ok_or_else(|| ProjectError {
                code: "artifact_not_found",
                message: "persisted GeoPackage artifact was not found".into(),
                diagnostics: Vec::new(),
            })?;
        let revision_id = self
            .store
            .resolve(snapshot, file.entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| ProjectError {
                code: "source_not_in_snapshot",
                message: "GeoPackage source is absent from the requested snapshot".into(),
                diagnostics: Vec::new(),
            })?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| ProjectError {
                code: "source_revision_missing",
                message: "GeoPackage source revision is unavailable".into(),
                diagnostics: Vec::new(),
            })?;
        let reader = GeoPackageReader::open(revision.payload(), dataset_semantic_id(file.entity))
            .map_err(|_| ProjectError {
            code: "geopackage_invalid",
            message: "persisted GeoPackage could not be read".into(),
            diagnostics: Vec::new(),
        })?;
        reader
            .feature_page(layer, offset, limit)
            .map_err(|_| ProjectError {
                code: "geopackage_page_invalid",
                message: "GeoPackage layer or feature page is invalid".into(),
                diagnostics: Vec::new(),
            })
    }
}

/// Creates a structured geospatial source-read failure.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe diagnostic detail.
///
/// Project read error without source bytes.
///
/// # Returns
/// Structured project read error.
fn geo_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
