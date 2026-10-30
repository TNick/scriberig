//! Implements local TUI reads for shared rich-domain application queries.

macro_rules! application_query_methods {
    () => {
        /// Inspects one exact coordinate through the shared application query contract.
        ///
        /// # Parameters
        /// - `context`: Caller authorization context.
        /// - `request`: Exact project, snapshot, worksheet, row, and column.
        ///
        /// # Returns
        /// Typed cell details normalized to the protocol response shape.
        ///
        /// # Errors
        /// Returns authorization, request, storage, or decoding failures.
        fn inspect_spreadsheet_cell(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellQuery,
        ) -> Result<scriberig_protocol::SpreadsheetCellInspectResponse, ApplicationError> {
            self.service
                .inspect_spreadsheet_cell(context, request)
                .map(crate::view::spreadsheet::cell_inspection_from_application)
        }

        /// Reads an exact-snapshot worksheet page through the shared query service.
        ///
        /// # Parameters
        /// - `context`: Caller authorization context.
        /// - `request`: Exact project, snapshot, worksheet, range, and page bounds.
        ///
        /// # Returns
        /// Typed cells and continuation metadata normalized to the public protocol shape.
        ///
        /// # Errors
        /// Returns authorization, request, storage, or semantic decoding failures.
        fn inspect_spreadsheet_cells(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellsQuery,
        ) -> Result<scriberig_protocol::SpreadsheetCellsResponse, ApplicationError> {
            self.service
                .read_spreadsheet_cells(&context.clone().with_snapshot(request.snapshot), request)
                .map(crate::view::spreadsheet::from_application)
        }

        /// Reads a bounded artifact page through the shared application query contract.
        ///
        /// # Parameters
        /// - `context`: Caller and scoped capability context.
        /// - `project`: Stable project identity.
        /// - `offset`: Zero-based artifact offset.
        /// - `limit`: Maximum number of artifact rows.
        ///
        /// # Returns
        /// Terminal-safe artifact metadata from the authorized application operation.
        ///
        /// # Errors
        /// Returns authorization, availability, or request failures.
        fn inspect_artifact_page(
            &self,
            context: &ApplicationContext,
            project: scriberig_core::ProjectId,
            offset: usize,
            limit: usize,
        ) -> Result<crate::view::artifacts::ArtifactPageModel, ApplicationError> {
            self.service
                .inspect_artifacts(context, project, offset, limit)
                .map(crate::view::artifacts::ArtifactPageModel::from_application)
        }

        /// Reads a bounded artifact-member page through the shared application service.
        ///
        /// # Parameters
        /// - `context`: Caller and scoped capability context.
        /// - `project`: Stable project identity.
        /// - `artifact`: Stable semantic artifact identity.
        /// - `offset`: Zero-based member offset.
        /// - `limit`: Maximum number of member descriptors.
        ///
        /// # Returns
        /// Terminal-safe exact member evidence.
        ///
        /// # Errors
        /// Returns authorization, availability, or request failures.
        fn inspect_artifact_members(
            &self,
            context: &ApplicationContext,
            project: scriberig_core::ProjectId,
            artifact: scriberig_core::EntityId,
            offset: usize,
            limit: usize,
        ) -> Result<crate::view::ArtifactMembersModel, ApplicationError> {
            self.service
                .inspect_artifact_members(context, project, artifact, offset, limit)
                .map(crate::view::ArtifactMembersModel::from_application)
        }
    };
}
