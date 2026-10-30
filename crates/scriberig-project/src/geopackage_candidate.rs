//! Creates exact-base `GeoPackage` geometry candidates through shared changes.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_geo::{Feature, GeoPackageLayerInfo, GeoPackageReader, Geometry};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::{
    ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange, ProjectSourceChangeKind,
    support::adapters::geopackage::dataset_semantic_id,
};

impl ProjectService {
    /// Proposes one exact-snapshot feature geometry replacement.
    ///
    /// The operation resolves the layer schema and feature identity against the
    /// exact source artifact revision, renders complete staged `GeoPackage` bytes,
    /// and persists them through the shared project candidate workflow. It does
    /// not mutate a live file.
    ///
    /// # Parameters
    /// - `project`: Project owning the `GeoPackage` source.
    /// - `base_snapshot`: Exact active semantic snapshot for the operation.
    /// - `path`: Exact project-relative `GeoPackage` path.
    /// - `layer`: Stable semantic layer identity.
    /// - `feature`: Stable semantic feature identity.
    /// - `geometry`: Replacement geometry in the layer CRS.
    ///
    /// # Returns
    /// Durable source candidate for validation, review, and shared apply.
    ///
    /// # Errors
    /// Returns an error when the base is stale, target is not in the exact source,
    /// geometry is invalid, or the package cannot be safely staged.
    pub fn propose_geopackage_feature_geometry(
        &self,
        project: ProjectId,
        base_snapshot: SnapshotId,
        path: &str,
        layer: EntityId,
        feature: EntityId,
        geometry: &Geometry,
    ) -> Result<ProjectCandidate, ProjectError> {
        let workspace = self.workspace(project)?;
        if workspace.snapshot != base_snapshot {
            return Err(geo_candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let ownership = self
            .store
            .project_entity(project, layer)
            .map_err(crate::errors::storage_error)?
            .filter(|record| record.kind == "geo.layer" && record.relative_path == path)
            .ok_or_else(|| {
                geo_candidate_error("geo_layer_not_found", "layer is not owned by this source")
            })?;
        let layer_revision = self
            .store
            .resolve(base_snapshot, layer)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                geo_candidate_error("geo_layer_not_in_snapshot", "layer is absent from base")
            })?;
        let layer_record = self
            .store
            .get(layer_revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                geo_candidate_error("geo_layer_revision_missing", "layer revision is missing")
            })?;
        let expected_layer: GeoPackageLayerInfo = serde_json::from_slice(layer_record.payload())
            .map_err(|_| geo_candidate_error("geo_layer_invalid", "layer schema is malformed"))?;
        let source = super::candidate::project_files_at(&self.store, project, base_snapshot)?
            .into_iter()
            .find(|file| {
                file.relative_path == ownership.relative_path && file.adapter == "geopackage"
            })
            .ok_or_else(|| {
                geo_candidate_error("geopackage_source_missing", "source is absent from base")
            })?;
        self.require_editable_source(project, path, source.digest)?;
        let dataset = dataset_semantic_id(source.entity);
        let reader = GeoPackageReader::open(&source.bytes, dataset)
            .map_err(|_| geo_candidate_error("geopackage_invalid", "source cannot be read"))?;
        let actual_layer = reader
            .layers()
            .iter()
            .find(|info| info.layer.id == layer)
            .ok_or_else(|| geo_candidate_error("geo_layer_not_found", "source layer is missing"))?;
        if actual_layer != &expected_layer {
            return Err(geo_candidate_error(
                "geo_layer_revision_mismatch",
                "persisted layer schema does not match exact source bytes",
            ));
        }
        let native_id = find_native_feature(&reader, layer, feature)?;
        let semantic_feature = self.resolve_geo_feature(base_snapshot, feature)?;
        let updated = scriberig_domain_geo::replace_feature_geometry_in_geopackage(
            &source.bytes,
            dataset,
            layer,
            &native_id,
            geometry,
        )
        .map_err(|_| {
            geo_candidate_error(
                "geopackage_edit_failed",
                "feature geometry or package cannot be safely edited",
            )
        })?;
        let mut candidate = self.propose_semantic_project_change_target(
            project,
            base_snapshot,
            vec![ProjectSourceChange {
                path: source.relative_path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "geo.replace_geometry",
            feature,
        )?;
        self.persist_feature_geometry_diff(
            &mut candidate,
            feature,
            semantic_feature.geometry.as_ref(),
            geometry,
        )?;
        Ok(candidate)
    }

    /// Resolves a typed feature payload from one exact semantic snapshot.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot containing the feature.
    /// - `feature`: Stable semantic feature identity.
    ///
    /// # Returns
    /// The decoded immutable feature revision.
    ///
    /// # Errors
    /// Returns an error when the feature or its retained revision is absent or invalid.
    fn resolve_geo_feature(
        &self,
        snapshot: SnapshotId,
        feature: EntityId,
    ) -> Result<Feature, ProjectError> {
        let revision = self
            .store
            .resolve(snapshot, feature)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                geo_candidate_error("geo_feature_not_in_snapshot", "feature is absent from base")
            })?;
        let record = self
            .store
            .get(revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                geo_candidate_error(
                    "geo_feature_revision_missing",
                    "feature revision is missing",
                )
            })?;
        serde_json::from_slice(record.payload()).map_err(|_| {
            geo_candidate_error("geo_feature_invalid", "feature revision is malformed")
        })
    }

    /// Attaches one typed geometry diff and persists its candidate.
    ///
    /// # Parameters
    /// - `candidate`: Staged candidate to complete and persist.
    /// - `feature`: Semantic feature being edited.
    /// - `before`: Prior optional geometry in the exact base revision.
    /// - `after`: Proposed replacement geometry.
    ///
    /// # Returns
    /// The persisted candidate for shared validation and application.
    ///
    /// # Errors
    /// Returns candidate, serialization, or storage errors.
    fn persist_feature_geometry_diff(
        &self,
        candidate: &mut ProjectCandidate,
        feature: EntityId,
        before: Option<&Geometry>,
        after: &Geometry,
    ) -> Result<(), ProjectError> {
        candidate
            .semantic_changes
            .push(crate::SemanticCandidateChange {
                target: feature,
                property: "feature.geometry".to_owned(),
                before: serde_json::to_value(before).map_err(|_| {
                    geo_candidate_error("geo_diff_invalid", "prior geometry is invalid")
                })?,
                after: serde_json::to_value(after).map_err(|_| {
                    geo_candidate_error("geo_diff_invalid", "proposed geometry is invalid")
                })?,
            });
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(())
    }
}

/// Resolves one semantic feature handle through bounded exact-layer pages.
///
/// # Parameters
/// - `reader`: Reader bound to exact `GeoPackage` source bytes.
/// - `layer`: Stable semantic layer identity.
/// - `feature`: Stable semantic feature identity to locate.
///
/// # Returns
/// Provider-native row ID used only by the `GeoPackage` writer.
///
/// # Errors
/// Returns an error when the target is absent or has no stable native ID.
pub(super) fn find_native_feature(
    reader: &GeoPackageReader,
    layer: EntityId,
    feature: EntityId,
) -> Result<String, ProjectError> {
    let mut offset = 0;
    loop {
        let page = reader
            .feature_page(layer, offset, 1000)
            .map_err(|_| geo_candidate_error("geopackage_page_failed", "feature page failed"))?;
        if let Some(target) = page
            .features
            .iter()
            .find(|candidate| candidate.id == feature)
        {
            return target.native_id.clone().ok_or_else(|| {
                geo_candidate_error("geo_feature_native_id_missing", "feature has no native ID")
            });
        }
        if !page.has_more {
            break;
        }
        offset = offset.checked_add(1000).ok_or_else(|| {
            geo_candidate_error("geopackage_offset_overflow", "feature page offset overflow")
        })?;
    }
    Err(geo_candidate_error(
        "geo_feature_not_found",
        "feature is absent from the exact layer revision",
    ))
}

/// Creates a structured geospatial candidate failure.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe explanatory detail.
///
/// # Returns
/// Project error without source bytes.
pub(super) fn geo_candidate_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
