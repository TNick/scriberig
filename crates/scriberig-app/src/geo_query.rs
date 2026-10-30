//! Defines and authorizes shared geospatial feature queries.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_geo::GeoPredicate;
use scriberig_project::{
    GeoPackageFeatureQuery as ProjectGeoFeatureQuery, GeoPackageFeatureQueryPage,
};

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Requests one bounded geospatial query against an exact project snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoFeatureQuery {
    /// Stable owning project identity.
    pub project: ProjectId,
    /// Exact immutable semantic snapshot.
    pub snapshot: SnapshotId,
    /// Exact project-relative `GeoPackage` source path.
    pub path: String,
    /// Stable semantic layer identity.
    pub layer: EntityId,
    /// Typed attribute or explicit-CRS spatial predicate.
    pub predicate: GeoPredicate,
    /// Zero-based matching result offset.
    pub offset: usize,
    /// Maximum results, from one through one thousand.
    pub limit: usize,
}

/// Returns bounded semantic feature summaries with exact query provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoFeatureQueryResult {
    /// Project owning the queried source.
    pub project: ProjectId,
    /// Exact query result and provider provenance.
    pub page: GeoPackageFeatureQueryPage,
}

impl<B> ApplicationService<B> {
    /// Executes a bounded geospatial query under an exact-snapshot Inspect grant.
    ///
    /// # Parameters
    /// - `context`: Caller grants and exact application context.
    /// - `request`: Project, snapshot, source, layer, predicate, and page bounds.
    ///
    /// # Returns
    /// Snapshot-bound matching feature summaries and source provenance.
    ///
    /// # Errors
    /// Returns authorization, request, source, predicate, or provider errors.
    pub fn query_geospatial_features(
        &self,
        context: &ApplicationContext,
        request: &GeoFeatureQuery,
    ) -> Result<GeoFeatureQueryResult, ApplicationError> {
        if !(1..=1000).contains(&request.limit) {
            return Err(ApplicationError::InvalidRequest {
                operation: "geo.features.query".to_owned(),
                message: "geospatial query limit must be between one and one thousand".to_owned(),
            });
        }
        authorize_geo_query(context, request.project, request.snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "geo.features.query".to_owned(),
            }
        })?;
        let page = service
            .query_geopackage_features(&ProjectGeoFeatureQuery {
                project: request.project,
                path: request.path.clone(),
                snapshot: request.snapshot,
                layer: request.layer,
                predicate: request.predicate.clone(),
                offset: request.offset,
                limit: request.limit,
            })
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "geo.features.query".to_owned(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(GeoFeatureQueryResult {
            project: request.project,
            page,
        })
    }
}

/// Requires exact project-scoped Inspect authority for a geospatial query.
///
/// # Parameters
/// - `context`: Caller grants and requested snapshot.
/// - `project`: Project whose source is being queried.
/// - `snapshot`: Exact semantic snapshot used for source resolution.
///
/// # Returns
/// Unit when the exact query scope is authorized.
///
/// # Errors
/// Returns a request error for snapshot mismatch or permission denied without a grant.
pub(crate) fn authorize_geo_query(
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
) -> Result<(), ApplicationError> {
    if context.snapshot != Some(snapshot) {
        return Err(ApplicationError::InvalidRequest {
            operation: "geo.features.query".to_owned(),
            message: "application context snapshot does not match the query".to_owned(),
        });
    }
    let scope = CapabilityScope::Custom(format!("project:{project}"));
    if context.capabilities.permits(
        &CapabilityAction::Inspect,
        &scope,
        Some(snapshot),
        Some(OperationKind::Query),
    ) || context.capabilities.permits(
        &CapabilityAction::Inspect,
        &scope,
        Some(snapshot),
        Some(OperationKind::Inspect),
    ) {
        Ok(())
    } else {
        Err(ApplicationError::PermissionDenied {
            action: "Inspect (geo.features.query)".to_owned(),
            scope: format!("project:{project}"),
        })
    }
}
