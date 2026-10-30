//! Exposes authorized, bounded `GeoPackage` catalog and feature reads.

use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_geo::{FeaturePage, GeoDataset};
use scriberig_project::GeoPackageLayerSummary;

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, CapabilityAction, CapabilityScope,
    OperationKind,
};

/// Contains bounded feature rows read under an exact semantic snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoFeaturePage {
    /// Project owning the persisted source evidence.
    pub project: ProjectId,
    /// Exact semantic snapshot authorizing this read.
    pub snapshot: scriberig_core::SnapshotId,
    /// Stable semantic layer identity.
    pub layer: EntityId,
    /// Bounded provider-neutral feature rows.
    pub page: FeaturePage,
}

/// Contains a bounded exact-snapshot `GeoPackage` dataset and layer catalog page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeoPackageCatalogPage {
    /// Project owning the persisted `GeoPackage` source.
    pub project: ProjectId,
    /// Exact snapshot used to resolve the source revision.
    pub snapshot: scriberig_core::SnapshotId,
    /// Stable logical dataset identity.
    pub dataset: GeoDataset,
    /// Exact source member digest.
    pub source_digest: [u8; 32],
    /// Ordered bounded layer summaries.
    pub layers: Vec<GeoPackageLayerSummary>,
    /// Total layer count before pagination.
    pub total: usize,
    /// Whether another layer page is available.
    pub has_more: bool,
}

impl<B> ApplicationService<B> {
    /// Reads bounded `GeoPackage` layer metadata under an explicit Inspect grant.
    ///
    /// # Parameters
    /// - `context`: Authorized request context bound to an exact snapshot.
    /// - `project`: Project whose persisted scan owns the `GeoPackage`.
    /// - `path`: Exact project-relative path from the artifact manifest.
    /// - `offset`: Zero-based layer offset.
    /// - `limit`: Maximum of two hundred layer summaries.
    ///
    /// # Returns
    /// Snapshot-bound dataset identity, source digest, and a layer catalog page.
    ///
    /// # Errors
    /// Returns an authorization, request, catalog, or package read error.
    pub fn inspect_geopackage(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        path: &str,
        offset: usize,
        limit: usize,
    ) -> Result<GeoPackageCatalogPage, ApplicationError> {
        if !(1..=scriberig_project::MAX_GEOPACKAGE_LAYER_PAGE).contains(&limit) {
            return Err(ApplicationError::InvalidRequest {
                operation: "geo.catalog.inspect".into(),
                message: "GeoPackage layer page limit must be between one and two hundred".into(),
            });
        }
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "geo.catalog.inspect".into(),
                message: "GeoPackage inspection requires an exact snapshot".into(),
            })?;
        authorize_geo_inspect(context, project, snapshot, "geo.catalog.inspect")?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "geo.catalog.inspect".into(),
            }
        })?;
        let catalog = service
            .geopackage_catalog(project, path, snapshot, offset, limit)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "geo.catalog.inspect".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(GeoPackageCatalogPage {
            project,
            snapshot,
            dataset: catalog.dataset,
            source_digest: catalog.source_digest,
            layers: catalog.layers,
            total: catalog.total,
            has_more: catalog.has_more,
        })
    }

    /// Reads bounded `GeoPackage` features under an explicit Inspect grant.
    ///
    /// # Parameters
    /// - `context`: Authorized request context bound to an exact snapshot.
    /// - `project`: Project whose persisted scan owns the `GeoPackage`.
    /// - `path`: Exact project-relative path from the artifact manifest.
    /// - `layer`: Stable semantic layer identity.
    /// - `offset`: Zero-based row offset.
    /// - `limit`: Page size from one through one thousand.
    ///
    /// # Returns
    /// Snapshot-bound provider-neutral feature page.
    ///
    /// # Errors
    /// Returns an authorization, request, catalog, or package read error.
    pub fn read_geopackage_features(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        path: &str,
        layer: EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<GeoFeaturePage, ApplicationError> {
        if !(1..=1000).contains(&limit) {
            return Err(ApplicationError::InvalidRequest {
                operation: "geo.features.read".into(),
                message: "feature page limit must be between one and one thousand".into(),
            });
        }
        let snapshot = context
            .snapshot
            .ok_or_else(|| ApplicationError::InvalidRequest {
                operation: "geo.features.read".into(),
                message: "feature reads require an exact snapshot".into(),
            })?;
        authorize_geo_inspect(context, project, snapshot, "geo.features.read")?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "geo.features.read".into(),
            }
        })?;
        let page = service
            .geopackage_feature_page(project, path, snapshot, layer, offset, limit)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "geo.features.read".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(GeoFeaturePage {
            project,
            snapshot,
            layer,
            page,
        })
    }
}

/// Enforces project-scoped Inspect authority for an exact semantic snapshot.
///
/// # Parameters
/// - `context`: Authorized request context.
/// - `project`: Project whose source is being read.
/// - `snapshot`: Exact source snapshot.
/// - `operation`: Stable operation name used for denial reporting.
///
/// # Returns
/// Unit when the grant allows the requested read.
///
/// # Errors
/// Returns permission denied when Inspect authority is absent.
fn authorize_geo_inspect(
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: scriberig_core::SnapshotId,
    operation: &'static str,
) -> Result<(), ApplicationError> {
    let scope = CapabilityScope::Custom(format!("project:{project}"));
    if context.capabilities.permits(
        &CapabilityAction::Inspect,
        &scope,
        Some(snapshot),
        Some(OperationKind::Inspect),
    ) {
        return Ok(());
    }
    Err(ApplicationError::PermissionDenied {
        action: format!("Inspect ({operation})"),
        scope: format!("project:{project}"),
    })
}
