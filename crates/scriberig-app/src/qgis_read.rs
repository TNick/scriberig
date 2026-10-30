//! Authorizes exact-snapshot QGIS project and layer-reference reads.

use scriberig_core::{ProjectId, SnapshotId};
use scriberig_project::QgisProjectRead;

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, geo_query::authorize_geo_query,
};

/// Requests one exact project-relative QGIS project model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QgisProjectQuery {
    /// Stable owning project identity.
    pub project: ProjectId,
    /// Exact immutable semantic snapshot.
    pub snapshot: SnapshotId,
    /// Exact project-relative `.qgs` or `.qgz` source path.
    pub path: String,
}

/// Returns project references bound to their exact snapshot and semantic revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QgisProjectResult {
    /// Stable owning Scriberig project.
    pub owner: ProjectId,
    /// Exact provider-neutral QGIS semantic read.
    pub qgis: QgisProjectRead,
}

impl<B> ApplicationService<B> {
    /// Reads an exact-snapshot QGIS project under project-scoped Inspect authority.
    ///
    /// # Parameters
    /// - `context`: Caller grants and exact application context.
    /// - `request`: Project, snapshot, and source path.
    ///
    /// # Returns
    /// QGIS layer references, linked `GeoLayer` IDs, diagnostics, and revision identity.
    ///
    /// # Errors
    /// Returns authorization, request, or persisted project read errors.
    pub fn inspect_qgis_project(
        &self,
        context: &ApplicationContext,
        request: &QgisProjectQuery,
    ) -> Result<QgisProjectResult, ApplicationError> {
        authorize_geo_query(context, request.project, request.snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "qgis.project.inspect".into(),
            }
        })?;
        let qgis = service
            .qgis_project(request.project, &request.path, request.snapshot)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "qgis.project.inspect".into(),
                message: format!("{}: {}", error.code, error.message),
            })?;
        Ok(QgisProjectResult {
            owner: request.project,
            qgis,
        })
    }
}
