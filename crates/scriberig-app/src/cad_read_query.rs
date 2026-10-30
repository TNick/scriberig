//! Exposes authorized CAD search and filtered reads to product surfaces.

use scriberig_core::{EntityId, ProjectId, SnapshotId};

use crate::{
    ApplicationContext, ApplicationError, ApplicationService, cad_candidate::authorize_cad_inspect,
};

/// Describes an exact-snapshot CAD annotation text query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadTextSearchRequest {
    /// Project owning the drawing.
    pub project: ProjectId,
    /// Exact immutable snapshot to inspect.
    pub snapshot: SnapshotId,
    /// Stable semantic drawing identity.
    pub drawing: EntityId,
    /// Non-empty text substring to find.
    pub query: String,
    /// Zero-based matching result offset.
    pub offset: usize,
    /// Maximum result page size.
    pub limit: usize,
}

/// Describes an exact-snapshot CAD layer entity query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadLayerEntitiesRequest {
    /// Project owning the drawing.
    pub project: ProjectId,
    /// Exact immutable snapshot to inspect.
    pub snapshot: SnapshotId,
    /// Stable semantic drawing identity.
    pub drawing: EntityId,
    /// Stable layer identity in the drawing.
    pub layer: EntityId,
    /// Zero-based offset among matching entities.
    pub offset: usize,
    /// Requested page size.
    pub limit: usize,
}

impl<B> ApplicationService<B> {
    /// Reads entities intersecting bounds with project-scoped Inspect authority.
    ///
    /// # Parameters
    /// # Parameters
    /// - `context`: Caller context bound to the requested exact snapshot.
    /// - `request`: Snapshot, drawing space, bounds, and page parameters.
    ///
    /// # Returns
    /// Bounded semantic matches with extent completeness metadata.
    ///
    /// # Errors
    /// Returns authorization, unsupported-operation, or project query errors.
    pub fn read_cad_entities_intersecting_bounds(
        &self,
        context: &ApplicationContext,
        request: scriberig_project::CadBoundsQueryRequest,
    ) -> Result<scriberig_project::CadBoundsEntityPage, ApplicationError> {
        if context.snapshot != Some(request.snapshot) {
            return Err(ApplicationError::InvalidRequest {
                operation: "cad.entities.bounds".into(),
                message: "CAD bounds query requires the exact requested snapshot".into(),
            });
        }
        authorize_cad_inspect(context, request.project, request.snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "cad.entities.bounds".into(),
            }
        })?;
        service
            .cad_entities_intersecting_bounds(request)
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "cad.entities.bounds".into(),
                message: format!("{}: {}", error.code, error.message),
            })
    }

    /// Finds CAD annotation text with project-scoped Inspect authority.
    ///
    /// # Parameters
    /// - `context`: Caller context bound to the exact requested snapshot.
    /// - `request`: Exact-snapshot query and page parameters.
    ///
    /// # Returns
    /// Bounded text matches with stable CAD entity identities.
    ///
    /// # Errors
    /// Returns authorization, unsupported-operation, or project query errors.
    pub fn find_cad_text(
        &self,
        context: &ApplicationContext,
        request: &CadTextSearchRequest,
    ) -> Result<scriberig_project::CadTextSearch, ApplicationError> {
        if context.snapshot != Some(request.snapshot) {
            return Err(ApplicationError::InvalidRequest {
                operation: "cad.text.find".into(),
                message: "CAD text search requires the exact requested snapshot".into(),
            });
        }
        authorize_cad_inspect(context, request.project, request.snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "cad.text.find".into(),
            }
        })?;
        service
            .find_cad_text(
                request.project,
                request.snapshot,
                request.drawing,
                &request.query,
                request.offset,
                request.limit,
            )
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "cad.text.find".into(),
                message: format!("{}: {}", error.code, error.message),
            })
    }

    /// Reads a bounded CAD entity page filtered to one semantic layer.
    ///
    /// # Parameters
    /// - `context`: Caller context bound to the exact requested snapshot.
    /// - `request`: Exact-snapshot layer query and page parameters.
    ///
    /// # Returns
    /// Bounded semantic entities and continuation metadata.
    ///
    /// # Errors
    /// Returns authorization, unsupported-operation, or project query errors.
    pub fn read_cad_entities_by_layer(
        &self,
        context: &ApplicationContext,
        request: &CadLayerEntitiesRequest,
    ) -> Result<scriberig_project::CadEntityPage, ApplicationError> {
        if context.snapshot != Some(request.snapshot) {
            return Err(ApplicationError::InvalidRequest {
                operation: "cad.entities.by_layer".into(),
                message: "CAD layer query requires the exact requested snapshot".into(),
            });
        }
        authorize_cad_inspect(context, request.project, request.snapshot)?;
        let service = self.project_service.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "cad.entities.by_layer".into(),
            }
        })?;
        service
            .cad_entities_by_layer(
                request.project,
                request.snapshot,
                request.drawing,
                request.layer,
                request.offset,
                request.limit,
            )
            .map_err(|error| ApplicationError::InvalidRequest {
                operation: "cad.entities.by_layer".into(),
                message: format!("{}: {}", error.code, error.message),
            })
    }
}
