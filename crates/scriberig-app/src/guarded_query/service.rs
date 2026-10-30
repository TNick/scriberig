//! Implements shared query forwarding for the guarded application provider.

use super::GuardedQueryService;
use crate::{
    ApplicationContext, ApplicationError, CapabilityAction, CapabilityScope, InspectRequest,
    InspectResult, QueryService,
};

impl<S: QueryService> QueryService for GuardedQueryService<S> {
    /// Delegates geospatial query authorization to its project-scoped application operation.
    ///
    /// # Parameters
    /// - `self`: Guarded query provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Exact project, snapshot, layer, predicate, and bounds.
    ///
    /// # Returns
    /// The authorized bounded feature query result.
    ///
    /// # Errors
    /// Returns authorization, query, source, or provider errors.
    fn query_geospatial_features(
        &self,
        context: &ApplicationContext,
        request: &crate::GeoFeatureQuery,
    ) -> Result<crate::GeoFeatureQueryResult, ApplicationError> {
        self.inner.query_geospatial_features(context, request)
    }

    /// Delegates QGIS project reads through the guarded shared query service.
    ///
    /// # Parameters
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Exact project, snapshot, and QGIS source path.
    ///
    /// # Returns
    /// QGIS references and resolved linked layer identities.
    ///
    /// # Errors
    /// Returns the inner query service error.
    fn inspect_qgis_project(
        &self,
        context: &ApplicationContext,
        request: &crate::QgisProjectQuery,
    ) -> Result<crate::QgisProjectResult, ApplicationError> {
        self.inner.inspect_qgis_project(context, request)
    }

    /// Delegates artifact inspection to the application operation that owns its project grant.
    ///
    /// # Parameters
    /// - `self`: Guarded query provider.
    /// - `context`: Caller grants and exact request context.
    /// - `project`: Stable project identity.
    /// - `offset`: Zero-based artifact offset.
    /// - `limit`: Maximum number of artifact summaries.
    ///
    /// # Returns
    /// The authorized bounded artifact page.
    ///
    /// # Errors
    /// Returns authorization, request, catalog, availability, or unsupported-operation errors.
    fn inspect_artifacts(
        &self,
        context: &ApplicationContext,
        project: scriberig_core::ProjectId,
        offset: usize,
        limit: usize,
    ) -> Result<crate::ArtifactManifestPage, ApplicationError> {
        self.inner
            .inspect_artifacts(context, project, offset, limit)
    }

    /// Delegates artifact-member inspection to the operation that owns project Inspect authority.
    ///
    /// # Parameters
    /// - `self`: Guarded query provider.
    /// - `context`: Caller grants and request context.
    /// - `project`: Stable project identity.
    /// - `artifact`: Stable semantic artifact identity.
    /// - `offset`: Zero-based member offset.
    /// - `limit`: Maximum member page size.
    ///
    /// # Returns
    /// The authorized bounded member page.
    ///
    /// # Errors
    /// Returns authorization, request, catalog, availability, or unsupported-operation errors.
    fn inspect_artifact_members(
        &self,
        context: &ApplicationContext,
        project: scriberig_core::ProjectId,
        artifact: scriberig_core::EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<crate::ArtifactMembers, ApplicationError> {
        self.inner
            .inspect_artifact_members(context, project, artifact, offset, limit)
    }

    /// Delegates spreadsheet reads to the application operation that owns the project grant.
    ///
    /// # Parameters
    /// - `self`: Guarded query provider.
    /// - `context`: Caller grants for the operation.
    /// - `request`: Exact project, snapshot, worksheet, range, and page bounds.
    ///
    /// # Returns
    /// The authorized snapshot-bound cell page.
    ///
    /// # Errors
    /// Returns authorization, request, storage, or decoding failures from the shared operation.
    fn read_spreadsheet_cells(
        &self,
        context: &ApplicationContext,
        request: &crate::SpreadsheetCellsQuery,
    ) -> Result<crate::SpreadsheetCells, ApplicationError> {
        self.inner.read_spreadsheet_cells(context, request)
    }

    /// Delegates coordinate inspection to the application operation that owns the grant.
    ///
    /// # Parameters
    /// - `self`: Guarded query provider.
    /// - `context`: Caller grants for the operation.
    /// - `request`: Exact project, snapshot, worksheet, and coordinate.
    ///
    /// # Returns
    /// The authorized cell inspection, including sparse blank coordinates.
    ///
    /// # Errors
    /// Returns authorization, request, storage, or decoding failures from the shared operation.
    fn inspect_spreadsheet_cell(
        &self,
        context: &ApplicationContext,
        request: &crate::SpreadsheetCellQuery,
    ) -> Result<crate::SpreadsheetCellInspection, ApplicationError> {
        self.inner.inspect_spreadsheet_cell(context, request)
    }

    /// Authorizes and delegates one application inspection.
    ///
    /// - `self`: Guarded provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Entity inspection request.
    ///
    /// The provider's inspection result after authorization.
    ///
    /// Returns a structured permission denial or provider error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        let scope = CapabilityScope::Entity(request.entity);
        Self::authorize(
            context,
            &CapabilityAction::Inspect,
            &scope,
            match request.snapshot {
                crate::SnapshotResolution::At(snapshot) => Some(snapshot),
                _ => context.snapshot,
            },
            crate::OperationKind::Inspect,
        )?;
        let result = self.inner.inspect(context, request)?;
        Ok(result)
    }

    /// Authorizes and delegates an inspection while preserving cancellation.
    ///
    /// - `self`: Guarded provider.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Entity inspection request.
    /// - `cancelled`: Caller-owned cancellation flag.
    ///
    /// The authorized inspection result when the read completes.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns cancellation, authorization denial, or the inner query error.
    fn inspect_cancellable(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<InspectResult, ApplicationError> {
        let scope = CapabilityScope::Entity(request.entity);
        Self::authorize(
            context,
            &CapabilityAction::Inspect,
            &scope,
            match request.snapshot {
                crate::SnapshotResolution::At(snapshot) => Some(snapshot),
                _ => context.snapshot,
            },
            crate::OperationKind::Inspect,
        )?;
        self.inner.inspect_cancellable(context, request, cancelled)
    }
}
