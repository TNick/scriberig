//! Creates exact-base schema-checked `GeoPackage` attribute candidates.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_geo::{Feature, GeoPackageLayerInfo, GeoPackageReader, GeoValue};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::ProjectFileRecord;

use crate::{
    ProjectCandidate, ProjectError, ProjectService, ProjectSourceChange, ProjectSourceChangeKind,
    geopackage_candidate::{find_native_feature, geo_candidate_error},
    support::adapters::geopackage::dataset_semantic_id,
};

/// Specifies one exact-base feature attribute replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoPackageFeatureAttributeEdit {
    /// Project owning the `GeoPackage` source.
    pub project: ProjectId,
    /// Exact active semantic snapshot for the operation.
    pub base_snapshot: SnapshotId,
    /// Exact project-relative source path.
    pub path: String,
    /// Stable semantic layer identity.
    pub layer: EntityId,
    /// Stable semantic feature identity.
    pub feature: EntityId,
    /// Authored schema field name.
    pub field: String,
    /// Typed replacement field value.
    pub value: GeoValue,
}

impl ProjectService {
    /// Proposes one schema-checked feature attribute replacement at an exact snapshot.
    ///
    /// The operation resolves layer schema and feature identity against the exact
    /// source revision, renders staged package bytes, and persists them through
    /// the shared candidate workflow without mutating a live file.
    ///
    /// # Parameters
    /// - `edit`: Project, exact source snapshot, target, field, and replacement value.
    ///
    /// # Returns
    /// Durable source candidate for validation, review, and shared apply.
    ///
    /// # Errors
    /// Returns stale-base, target, schema, or safe-writer errors.
    pub fn propose_geopackage_feature_attribute(
        &self,
        edit: &GeoPackageFeatureAttributeEdit,
    ) -> Result<ProjectCandidate, ProjectError> {
        let GeoPackageFeatureAttributeEdit {
            project,
            base_snapshot,
            path,
            layer,
            feature,
            field,
            value,
        } = edit;
        let (project, base_snapshot, layer, feature) = (*project, *base_snapshot, *layer, *feature);
        let workspace = self.workspace(project)?;
        if workspace.snapshot != base_snapshot {
            return Err(geo_candidate_error(
                "stale_candidate",
                "requested base is not active",
            ));
        }
        let (source, native_id) =
            self.resolve_feature_source(project, base_snapshot, path, layer, feature)?;
        let dataset = dataset_semantic_id(source.entity);
        let feature_record = self
            .store
            .resolve(base_snapshot, feature)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                geo_candidate_error("geo_feature_not_in_snapshot", "feature is absent from base")
            })?;
        let feature_record = self
            .store
            .get(feature_record)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                geo_candidate_error(
                    "geo_feature_revision_missing",
                    "feature revision is missing",
                )
            })?;
        let semantic_feature: Feature =
            serde_json::from_slice(feature_record.payload()).map_err(|_| {
                geo_candidate_error("geo_feature_invalid", "feature revision is malformed")
            })?;
        let before = semantic_feature.attributes.get(field).ok_or_else(|| {
            geo_candidate_error("geo_field_not_found", "field is absent from feature")
        })?;
        let updated = scriberig_domain_geo::replace_feature_attribute_in_geopackage(
            &source.bytes,
            dataset,
            layer,
            &native_id,
            field,
            value,
        )
        .map_err(|error| {
            geo_candidate_error(
                "geopackage_attribute_edit_failed",
                &format!("feature attribute or package cannot be safely edited: {error:?}"),
            )
        })?;
        self.persist_feature_attribute_candidate(edit, source.relative_path, updated, before)
    }

    /// Resolves the source and native feature ID against an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Project owning the source.
    /// - `snapshot`: Exact semantic snapshot.
    /// - `path`: Project-relative source path.
    /// - `layer`: Semantic layer identity.
    /// - `feature`: Semantic feature identity.
    ///
    /// # Returns
    /// The retained source record and provider-native row ID for the target.
    ///
    /// # Errors
    /// Returns an error when ownership, source bytes, schema, or target cannot be resolved.
    fn resolve_feature_source(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        path: &str,
        layer: EntityId,
        feature: EntityId,
    ) -> Result<(ProjectFileRecord, String), ProjectError> {
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
            .resolve(snapshot, layer)
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
        let expected: GeoPackageLayerInfo = serde_json::from_slice(layer_record.payload())
            .map_err(|_| geo_candidate_error("geo_layer_invalid", "layer schema is malformed"))?;
        let source = super::candidate::project_files_at(&self.store, project, snapshot)?
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
        let actual = reader
            .layers()
            .iter()
            .find(|info| info.layer.id == layer)
            .ok_or_else(|| geo_candidate_error("geo_layer_not_found", "source layer is missing"))?;
        if actual != &expected {
            return Err(geo_candidate_error(
                "geo_layer_revision_mismatch",
                "persisted layer schema does not match exact source bytes",
            ));
        }
        let native_id = find_native_feature(&reader, layer, feature)?;
        Ok((source, native_id))
    }

    /// Persists staged attribute bytes and the exact typed value diff.
    ///
    /// # Parameters
    /// - `project`: Project owning the source artifact.
    /// - `base_snapshot`: Exact candidate base.
    /// - `path`: Project-relative source path.
    /// - `updated`: Fully staged `GeoPackage` bytes.
    /// - `feature`: Semantic feature being edited.
    /// - `field`: Authored schema field name.
    /// - `before`: Prior value resolved from the exact feature revision.
    /// - `after`: Proposed typed attribute value.
    /// - `edit`: Exact semantic feature edit accepted by the candidate operation.
    ///
    /// # Returns
    /// The persisted candidate for shared validation and application.
    ///
    /// # Errors
    /// Returns candidate, serialization, or storage errors.
    fn persist_feature_attribute_candidate(
        &self,
        edit: &GeoPackageFeatureAttributeEdit,
        path: String,
        updated: Vec<u8>,
        before: &GeoValue,
    ) -> Result<ProjectCandidate, ProjectError> {
        let mut candidate = self.propose_semantic_project_change_target(
            edit.project,
            edit.base_snapshot,
            vec![ProjectSourceChange {
                path,
                kind: ProjectSourceChangeKind::Update,
                content: Some(updated),
            }],
            "geo.set_attribute",
            edit.feature,
        )?;
        candidate
            .semantic_changes
            .push(crate::SemanticCandidateChange {
                target: edit.feature,
                property: format!("feature.attribute.{}", edit.field),
                before: serde_json::to_value(before).map_err(|_| {
                    geo_candidate_error("geo_diff_invalid", "prior value is invalid")
                })?,
                after: serde_json::to_value(&edit.value).map_err(|_| {
                    geo_candidate_error("geo_diff_invalid", "proposed value is invalid")
                })?,
            });
        self.store
            .put_project_candidate(&candidate.record())
            .map_err(crate::errors::storage_error)?;
        Ok(candidate)
    }
}
