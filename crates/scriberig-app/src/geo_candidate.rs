//! Exposes authorized geospatial geometry candidates to product surfaces.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_geo::Geometry;
use scriberig_project::{CandidateValidation, GeoPackageFeatureAttributeEdit};

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Contains a durable `GeoPackage` feature geometry candidate without package bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoGeometryCandidate {
    /// Stable shared project candidate identity.
    pub id: String,
    /// Project owning the geospatial source.
    pub project: ProjectId,
    /// Exact snapshot against which the operation was proposed.
    pub base_snapshot: SnapshotId,
    /// Stable semantic feature identity.
    pub feature: EntityId,
    /// Stable semantic layer identity.
    pub layer: EntityId,
    /// Exact project-relative `GeoPackage` source path.
    pub path: String,
    /// Current candidate validation state.
    pub validation: CandidateValidation,
}

/// Contains one durable `GeoPackage` feature attribute candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoAttributeCandidate {
    /// Stable shared project candidate identity.
    pub id: String,
    /// Project owning the geospatial source.
    pub project: ProjectId,
    /// Exact snapshot against which the operation was proposed.
    pub base_snapshot: SnapshotId,
    /// Stable semantic feature identity.
    pub feature: EntityId,
    /// Stable semantic layer identity.
    pub layer: EntityId,
    /// Authored schema field name.
    pub field: String,
    /// Exact project-relative `GeoPackage` source path.
    pub path: String,
    /// Current candidate validation state.
    pub validation: CandidateValidation,
}

impl<B> ApplicationService<B> {
    /// Proposes a typed feature geometry edit under exact-snapshot Propose authority.
    ///
    /// # Parameters
    /// - `context`: Request context bound to the exact base snapshot and grant.
    /// - `project`: Project owning the `GeoPackage` source.
    /// - `path`: Exact project-relative `GeoPackage` path.
    /// - `layer`: Stable semantic layer identity.
    /// - `feature`: Stable semantic feature identity.
    /// - `geometry`: Replacement geometry in the layer CRS.
    ///
    /// # Returns
    /// Candidate metadata for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns authorization, stale-base, unsupported-target, or candidate errors.
    pub fn propose_geo_feature_geometry(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        path: &str,
        layer: EntityId,
        feature: EntityId,
        geometry: &Geometry,
    ) -> Result<GeoGeometryCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "geo.geometry.propose".into(),
                message: "geospatial edits require an exact snapshot".into(),
            })?;
        authorize_geo_proposal(context, project, layer, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "geo.geometry.propose".into(),
            }
        })?;
        let candidate = service
            .propose_geopackage_feature_geometry(project, snapshot, path, layer, feature, geometry)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "geo.geometry.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(GeoGeometryCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            feature,
            layer,
            path: candidate.path,
            validation: candidate.validation,
        })
    }

    /// Proposes one typed attribute edit under exact-snapshot Propose authority.
    ///
    /// # Parameters
    /// - `context`: Request context bound to the exact base snapshot and grant.
    /// - `edit`: Exact project source, feature, field, and replacement value.
    ///
    /// # Returns
    /// Candidate metadata for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns authorization, stale-base, schema, unsupported-target, or candidate errors.
    pub fn propose_geo_feature_attribute(
        &self,
        context: &ApplicationContext,
        edit: GeoPackageFeatureAttributeEdit,
    ) -> Result<GeoAttributeCandidate, ApplicationError> {
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "geo.attribute.propose".into(),
                message: "geospatial edits require an exact snapshot".into(),
            })?;
        authorize_geo_proposal(context, edit.project, edit.layer, snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "geo.attribute.propose".into(),
            }
        })?;
        let candidate = service
            .propose_geopackage_feature_attribute(&edit)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "geo.attribute.propose".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(GeoAttributeCandidate {
            id: candidate.id,
            project: candidate.project,
            base_snapshot: candidate.base_snapshot,
            feature: edit.feature,
            layer: edit.layer,
            field: edit.field,
            path: candidate.path,
            validation: candidate.validation,
        })
    }
}

/// Requires exact-snapshot Propose authority over the layer or its project.
///
/// A layer grant supports selection-scoped agent work; a project grant keeps
/// trusted product surfaces compatible with project-wide candidate review.
///
/// # Parameters
/// - `context`: Request context carrying Propose authority.
/// - `project`: Project owning the target layer.
/// - `layer`: Exact semantic layer being edited.
/// - `snapshot`: Immutable base snapshot.
///
/// # Returns
/// Unit when either the exact layer or owning project is authorized.
///
/// # Errors
/// Returns permission denied when neither scope has matching authority.
fn authorize_geo_proposal(
    context: &ApplicationContext,
    project: scriberig_core::ProjectId,
    layer: EntityId,
    snapshot: SnapshotId,
) -> Result<(), ApplicationError> {
    let project_scope = CapabilityScope::Custom(format!("project:{project}"));
    let layer_scope = CapabilityScope::Entity(layer);
    let authorized = [&layer_scope, &project_scope].into_iter().any(|scope| {
        context.capabilities.permits(
            &CapabilityAction::Propose,
            scope,
            Some(snapshot),
            Some(OperationKind::DomainOperation),
        )
    });
    if authorized {
        Ok(())
    } else {
        Err(ApplicationError::PermissionDenied {
            action: "Propose".into(),
            scope: format!("project:{project} layer:{layer}"),
        })
    }
}
