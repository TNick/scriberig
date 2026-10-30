//! Executes bounded geospatial predicates over exact persisted `GeoPackage` revisions.

use scriberig_core::{EntityId, ProjectId, RevisionId, SnapshotId};
use scriberig_domain_geo::{
    CoordinateReferenceSystem, Feature, FeatureCollection, GeoPackageReader, GeoPredicate,
};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{ProjectError, ProjectService, support::adapters::geopackage::dataset_semantic_id};

/// Describes how a source-backed feature query was evaluated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeoQueryStrategy {
    /// Scans bounded source pages and retains only the requested result page.
    StreamedSourcePages,
}

/// Records exact source and CRS provenance for a feature query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoQueryProvenance {
    /// Exact source revision resolved from the requested snapshot.
    pub source_revision: RevisionId,
    /// Digest of the exact source package bytes.
    pub source_digest: [u8; 32],
    /// Adapter used to evaluate the source query.
    pub provider: String,
    /// Provider-neutral layer CRS used to check spatial predicate scope.
    pub layer_crs: Option<CoordinateReferenceSystem>,
    /// Evaluation strategy used for this result.
    pub strategy: GeoQueryStrategy,
    /// Explicit statement that no coordinate transform was applied.
    pub coordinate_transform: Option<String>,
    /// Safe diagnostics describing execution constraints.
    pub diagnostics: Vec<String>,
}

/// Contains a bounded exact-snapshot feature query result and its provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoPackageFeatureQueryPage {
    /// Exact snapshot queried.
    pub snapshot: SnapshotId,
    /// Stable semantic layer queried.
    pub layer: EntityId,
    /// Matching semantic feature summaries for this page.
    pub features: Vec<Feature>,
    /// Whether another matching page exists.
    pub has_more: bool,
    /// Exact source and execution provenance.
    pub provenance: GeoQueryProvenance,
}

/// Defines the exact source scope and bounds for a `GeoPackage` feature query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoPackageFeatureQuery {
    /// Project owning the persisted source scan.
    pub project: ProjectId,
    /// Exact project-relative source path.
    pub path: String,
    /// Exact immutable source snapshot.
    pub snapshot: SnapshotId,
    /// Stable semantic layer identity.
    pub layer: EntityId,
    /// Typed attribute or explicit-CRS envelope predicate.
    pub predicate: GeoPredicate,
    /// Number of matching features to skip.
    pub offset: usize,
    /// Maximum result count from one through one thousand.
    pub limit: usize,
}

impl ProjectService {
    /// Queries features in an exact persisted source with bounded result memory.
    ///
    /// The provider is read in bounded pages. Predicate evaluation uses the shared
    /// Geo feature collection query contract, and no source mutation or implicit
    /// CRS conversion occurs.
    ///
    /// # Parameters
    /// - `request`: Exact source scope, predicate, and page bounds.
    ///
    /// # Returns
    /// Bounded matching feature summaries with source, snapshot, and CRS provenance.
    ///
    /// # Errors
    /// Returns source, snapshot, layer, page, predicate, or CRS validation errors.
    pub fn query_geopackage_features(
        &self,
        request: &GeoPackageFeatureQuery,
    ) -> Result<GeoPackageFeatureQueryPage, ProjectError> {
        if !(1..=1000).contains(&request.limit) {
            return Err(query_error(
                "invalid_page_size",
                "geospatial query limit must be between one and one thousand",
            ));
        }
        let file = self
            .store
            .project_files(request.project)
            .map_err(crate::errors::storage_debug_error)?
            .into_iter()
            .find(|file| file.relative_path == request.path && file.adapter == "geopackage")
            .ok_or_else(|| query_error("artifact_not_found", "GeoPackage source was not found"))?;
        let source_revision = self
            .store
            .resolve(request.snapshot, file.entity)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                query_error(
                    "source_not_in_snapshot",
                    "GeoPackage source is absent from the requested snapshot",
                )
            })?;
        let source = self
            .store
            .get(source_revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                query_error(
                    "source_revision_missing",
                    "GeoPackage source revision is unavailable",
                )
            })?;
        let reader = GeoPackageReader::open(source.payload(), dataset_semantic_id(file.entity))
            .map_err(|_| {
                query_error(
                    "geopackage_invalid",
                    "persisted GeoPackage could not be read",
                )
            })?;
        let layer_crs = reader
            .layers()
            .iter()
            .find(|info| info.layer.id == request.layer)
            .ok_or_else(|| query_error("geo_layer_not_found", "semantic layer is not in source"))?
            .layer
            .crs
            .clone();
        let (result_features, has_more) = collect_query_page(&reader, request, layer_crs.as_ref())?;
        Ok(GeoPackageFeatureQueryPage {
            snapshot: request.snapshot,
            layer: request.layer,
            features: result_features,
            has_more,
            provenance: GeoQueryProvenance {
                source_revision,
                source_digest: *blake3::hash(source.payload()).as_bytes(),
                provider: "geopackage".to_owned(),
                layer_crs,
                strategy: GeoQueryStrategy::StreamedSourcePages,
                coordinate_transform: None,
                diagnostics: vec![
                    "Query evaluated against the exact source revision without CRS transformation."
                        .to_owned(),
                ],
            },
        })
    }
}

/// Collects matching features incrementally and retains only one result page.
///
/// # Parameters
/// - `reader`: Reader for the exact persisted package revision.
/// - `request`: Predicate and result page bounds.
/// - `layer_crs`: Explicit CRS declared by the target layer.
///
/// # Returns
/// Bounded feature summaries and whether another result exists.
///
/// # Errors
/// Returns source page, semantic identity, predicate, or CRS errors.
fn collect_query_page(
    reader: &GeoPackageReader,
    request: &GeoPackageFeatureQuery,
    layer_crs: Option<&CoordinateReferenceSystem>,
) -> Result<(Vec<Feature>, bool), ProjectError> {
    let mut features = Vec::with_capacity(request.limit);
    let mut skipped = 0_usize;
    let mut source_offset = 0_usize;
    loop {
        let page = reader
            .feature_page(request.layer, source_offset, 1000)
            .map_err(|_| {
                query_error("geopackage_page_invalid", "source feature page is invalid")
            })?;
        let count = page.features.len();
        let has_source_more = page.has_more;
        let collection = FeatureCollection::new(page.features.clone(), 1000).map_err(|_| {
            query_error(
                "geo_features_invalid",
                "source feature identities are invalid",
            )
        })?;
        let matches = collection
            .query(&request.predicate, request.snapshot, layer_crs, 0, 1000)
            .map_err(|error| {
                query_error(
                    "geo_predicate_invalid",
                    &format!("geospatial predicate failed: {error:?}"),
                )
            })?;
        let by_id = page
            .features
            .into_iter()
            .map(|feature| (feature.id, feature))
            .collect::<std::collections::BTreeMap<_, _>>();
        for handle in matches.matches {
            if skipped < request.offset {
                skipped = skipped.saturating_add(1);
            } else if features.len() < request.limit {
                if let Some(feature) = by_id.get(&handle.feature) {
                    features.push(feature.clone());
                }
            } else {
                return Ok((features, true));
            }
        }
        if !has_source_more {
            return Ok((features, false));
        }
        source_offset = source_offset.saturating_add(count);
    }
}

/// Creates a safe source query error.
///
/// # Parameters
/// - `code`: Stable query error category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// Project error without exposing source bytes or database details.
fn query_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
