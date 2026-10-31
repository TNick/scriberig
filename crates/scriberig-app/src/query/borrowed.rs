//! Delegates shared query operations through borrowed providers.

use super::{
    InspectRequest, InspectResult, QueryService, SpreadsheetCellQuery, SpreadsheetCellsQuery,
};
use crate::{ApplicationContext, ApplicationError, ArtifactManifestPage};
use scriberig_core::ProjectId;
impl<S: QueryService + ?Sized> QueryService for &S {
    /// Delegates a geospatial query through a borrowed query service.
    ///
    /// # Parameters
    /// - `self`: Borrowed query service.
    /// - `context`: Caller grants and request context.
    /// - `request`: Exact project, snapshot, layer, predicate, and bounds.
    ///
    /// # Returns
    /// The delegated bounded feature query result.
    ///
    /// # Errors
    /// Returns the delegated application failure.
    fn query_geospatial_features(
        &self,
        context: &ApplicationContext,
        request: &crate::GeoFeatureQuery,
    ) -> Result<crate::GeoFeatureQueryResult, ApplicationError> {
        (*self).query_geospatial_features(context, request)
    }

    /// Delegates QGIS project inspection through a borrowed query service.
    ///
    /// # Parameters
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Exact project, snapshot, and QGIS source path.
    ///
    /// # Returns
    /// The delegated QGIS project and layer-reference semantics.
    ///
    /// # Errors
    /// Returns the delegated application failure.
    fn inspect_qgis_project(
        &self,
        context: &ApplicationContext,
        request: &crate::QgisProjectQuery,
    ) -> Result<crate::QgisProjectResult, ApplicationError> {
        (*self).inspect_qgis_project(context, request)
    }

    /// Delegates artifact inspection through a borrowed query service.
    ///
    /// # Parameters
    /// - `self`: Borrowed application query service.
    /// - `context`: Caller, capability, and snapshot context.
    /// - `project`: Stable project identity.
    /// - `offset`: Zero-based artifact offset.
    /// - `limit`: Maximum number of artifact summaries.
    ///
    /// # Returns
    /// The delegated bounded artifact page.
    ///
    /// # Errors
    /// Returns the delegated application failure.
    fn inspect_artifacts(
        &self,
        context: &ApplicationContext,
        project: ProjectId,
        offset: usize,
        limit: usize,
    ) -> Result<ArtifactManifestPage, ApplicationError> {
        (*self).inspect_artifacts(context, project, offset, limit)
    }

    /// Delegates a bounded spreadsheet read through a borrowed query service.
    ///
    /// # Parameters
    /// - `self`: Borrowed query service.
    /// - `context`: Caller grants for the operation.
    /// - `request`: Exact project, snapshot, worksheet, range, and page bounds.
    ///
    /// # Returns
    /// The delegated snapshot-bound cell page.
    ///
    /// # Errors
    /// Returns the delegated application failure.
    fn read_spreadsheet_cells(
        &self,
        context: &ApplicationContext,
        request: &SpreadsheetCellsQuery,
    ) -> Result<crate::SpreadsheetCells, ApplicationError> {
        (*self).read_spreadsheet_cells(context, request)
    }

    /// Delegates one exact-coordinate spreadsheet inspection through a borrowed service.
    ///
    /// # Parameters
    /// - `self`: Borrowed query service.
    /// - `context`: Caller grants for the operation.
    /// - `request`: Exact project, snapshot, worksheet, and coordinate.
    ///
    /// # Returns
    /// The delegated cell inspection, including a sparse blank result.
    ///
    /// # Errors
    /// Returns the delegated application failure.
    fn inspect_spreadsheet_cell(
        &self,
        context: &ApplicationContext,
        request: &SpreadsheetCellQuery,
    ) -> Result<crate::SpreadsheetCellInspection, ApplicationError> {
        (*self).inspect_spreadsheet_cell(context, request)
    }

    /// Delegates inspection through a borrowed query service.
    ///
    /// - `self`: Borrowed application query service.
    /// - `context`: Caller, capability, and snapshot context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// Snapshot-bound inspection metadata.
    ///
    /// Returns the delegated application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        (*self).inspect(context, request)
    }

    /// Delegates a cancellable inspection through a borrowed query service.
    ///
    /// - `self`: Borrowed application query service.
    /// - `context`: Caller, capability, and snapshot context.
    /// - `request`: Snapshot-aware inspection request.
    /// - `cancelled`: Caller-owned cancellation flag.
    ///
    /// Snapshot-bound inspection metadata when the read completes.
    ///
    /// # Returns
    /// The delegated inspection result.
    ///
    /// # Errors
    /// Returns cancellation or the delegated application failure.
    fn inspect_cancellable(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<InspectResult, ApplicationError> {
        (**self).inspect_cancellable(context, request, cancelled)
    }
}
