//! Implements shared semantic queries through the application service.

use super::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService,
    service_query_helpers::{inspect_result, query_request},
};
use crate::{InspectRequest, InspectResult, QueryService};

impl<B: ApplicationBackend> QueryService for ApplicationService<B> {
    /// Executes an authorized bounded geospatial query.
    ///
    /// # Parameters
    /// - `self`: Application query service.
    /// - `context`: Caller grants and request context.
    /// - `request`: Exact project, snapshot, layer, predicate, and bounds.
    ///
    /// # Returns
    /// Matching feature summaries and provider provenance.
    ///
    /// # Errors
    /// Returns authorization, query, source, or provider errors.
    fn query_geospatial_features(
        &self,
        context: &ApplicationContext,
        request: &crate::GeoFeatureQuery,
    ) -> Result<crate::GeoFeatureQueryResult, ApplicationError> {
        ApplicationService::query_geospatial_features(self, context, request)
    }

    /// Reads QGIS project references through the shared application query API.
    ///
    /// # Parameters
    /// - `self`: Shared query service.
    /// - `context`: Caller grants and exact request context.
    /// - `request`: Exact project, snapshot, and QGIS source path.
    ///
    /// # Returns
    /// QGIS references and resolved `GeoLayer` identities.
    ///
    /// # Errors
    /// Returns authorization or project read failures.
    fn inspect_qgis_project(
        &self,
        context: &ApplicationContext,
        request: &crate::QgisProjectQuery,
    ) -> Result<crate::QgisProjectResult, ApplicationError> {
        ApplicationService::inspect_qgis_project(self, context, request)
    }

    /// Reads project artifacts through the shared authorized application operation.
    ///
    /// # Parameters
    /// - `self`: Application service.
    /// - `context`: Caller and scoped capability context.
    /// - `project`: Stable project identity.
    /// - `offset`: Zero-based artifact offset.
    /// - `limit`: Maximum number of artifact summaries.
    ///
    /// # Returns
    /// A bounded deterministic artifact page.
    ///
    /// # Errors
    /// Returns authorization, request, catalog, or availability failures.
    fn inspect_artifacts(
        &self,
        context: &ApplicationContext,
        project: scriberig_core::ProjectId,
        offset: usize,
        limit: usize,
    ) -> Result<crate::ArtifactManifestPage, ApplicationError> {
        ApplicationService::inspect_artifacts(self, context, project, offset, limit)
    }

    /// Reads artifact members through the shared authorized application operation.
    ///
    /// # Parameters
    /// - `self`: Application service.
    /// - `context`: Caller and scoped capability context.
    /// - `project`: Stable project identity.
    /// - `artifact`: Stable artifact identity.
    /// - `offset`: Zero-based member offset.
    /// - `limit`: Maximum member page size.
    ///
    /// # Returns
    /// A bounded exact member page.
    ///
    /// # Errors
    /// Returns authorization, request, catalog, or availability failures.
    fn inspect_artifact_members(
        &self,
        context: &ApplicationContext,
        project: scriberig_core::ProjectId,
        artifact: scriberig_core::EntityId,
        offset: usize,
        limit: usize,
    ) -> Result<crate::ArtifactMembers, ApplicationError> {
        ApplicationService::inspect_artifact_members(
            self, context, project, artifact, offset, limit,
        )
    }

    /// Reads a bounded worksheet page through the shared authorized application operation.
    ///
    /// # Parameters
    /// - `self`: Application service.
    /// - `context`: Caller grants for the operation.
    /// - `request`: Exact project, snapshot, worksheet, range, and page bounds.
    ///
    /// # Returns
    /// Snapshot-bound typed cells and continuation metadata.
    ///
    /// # Errors
    /// Returns authorization, request, storage, or semantic decoding failures.
    fn read_spreadsheet_cells(
        &self,
        context: &ApplicationContext,
        request: &crate::SpreadsheetCellsQuery,
    ) -> Result<crate::SpreadsheetCells, ApplicationError> {
        ApplicationService::read_spreadsheet_cells(
            self,
            &context.clone().with_snapshot(request.snapshot),
            request.project,
            request.worksheet,
            request.range.as_ref(),
            request.offset,
            request.limit,
        )
    }

    /// Inspects one exact worksheet coordinate through the shared application operation.
    ///
    /// # Parameters
    /// - `self`: Application service.
    /// - `context`: Caller grants for the operation.
    /// - `request`: Exact project, snapshot, worksheet, and coordinate.
    ///
    /// # Returns
    /// Typed semantic cell data, or `None` for a sparse blank coordinate.
    ///
    /// # Errors
    /// Returns authorization, request, storage, or semantic decoding failures.
    fn inspect_spreadsheet_cell(
        &self,
        context: &ApplicationContext,
        request: &crate::SpreadsheetCellQuery,
    ) -> Result<crate::SpreadsheetCellInspection, ApplicationError> {
        ApplicationService::inspect_spreadsheet_cell(
            self,
            &context.clone().with_snapshot(request.snapshot),
            request.project,
            request.worksheet,
            request.row,
            request.column,
        )
    }

    /// Executes an application inspection request through the universal path.
    ///
    /// - `self`: Application service.
    /// - `context`: Caller and capability context.
    /// - `request`: Snapshot-aware application inspection request.
    ///
    /// Application inspection metadata in the CLI/server-facing shape.
    ///
    /// Returns a structured application failure when the operation fails or
    /// returns a non-inspection result.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        let current = match request.snapshot {
            crate::SnapshotResolution::Current => Some(self.backend.current_snapshot(context)?),
            _ => None,
        };
        let application_request = ApplicationRequest::Inspect(query_request(request, current)?);
        let result = self.execute(context, &application_request)?;
        let ApplicationResult::Inspect(inspection) = result else {
            return Err(ApplicationError::Internal {
                message: "inspect backend returned a different result kind".to_owned(),
            });
        };
        inspect_result(inspection, request.detail)
    }

    /// Executes inspection through the application backend's cancellable path.
    ///
    /// - `self`: Application service.
    /// - `context`: Caller and capability context.
    /// - `request`: Snapshot-aware application inspection request.
    /// - `cancelled`: Caller-owned cancellation flag.
    ///
    /// Application inspection metadata when the read completes.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns cancellation, authorization, storage, or result-kind failures.
    fn inspect_cancellable(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<InspectResult, ApplicationError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ApplicationError::OperationCancelled {
                operation: "inspect".to_owned(),
            });
        }
        let current = match request.snapshot {
            crate::SnapshotResolution::Current => Some(self.backend.current_snapshot(context)?),
            _ => None,
        };
        let application_request = ApplicationRequest::Inspect(query_request(request, current)?);
        let result = self.execute_cancellable(context, &application_request, cancelled)?;
        let ApplicationResult::Inspect(inspection) = result else {
            return Err(ApplicationError::Internal {
                message: "inspect backend returned a different result kind".to_owned(),
            });
        };
        inspect_result(inspection, request.detail)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
