//! Provides documents methods for the application operation adapter.

macro_rules! operation_documents_methods {
    () => {
        /// Reads a bounded document tree from the exact application snapshot.
        ///
        /// # Parameters
        /// - `context`: Caller grants and exact request context.
        /// - `request`: Stable project, snapshot, and document identity.
        ///
        /// # Returns
        /// Protocol-shaped semantic sections, paragraphs, and tables.
        ///
        /// # Errors
        /// Returns authorization, stale-snapshot, project-read, or normalization errors.
        fn inspect_document(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::DocumentTreeQuery,
        ) -> Result<scriberig_protocol::DocumentInspectResponse, ApplicationError> {
            self.service
                .inspect_document(context, request.project, request.snapshot, request.document)
                .map(crate::view::document::from_application)
        }

        /// Creates a paragraph candidate through the exact-snapshot application API.
        ///
        /// # Parameters
        /// - `context`: Caller grants and request context.
        /// - `request`: Project, snapshot, paragraph, and replacement text.
        ///
        /// # Returns
        /// Durable candidate metadata for shared review and apply.
        ///
        /// # Errors
        /// Returns authorization, stale-base, read-only fidelity, or candidate errors.
        fn propose_document_paragraph_edit(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::DocumentParagraphEditQuery,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            self.service.propose_document_paragraph_edit(
                &context.clone().with_snapshot(request.snapshot),
                request.project,
                request.paragraph,
                &request.replacement,
            )
        }

        /// Creates a cell candidate through the exact-snapshot application API.
        ///
        /// # Parameters
        /// - `context`: Caller grants and request context.
        /// - `request`: Project, snapshot, cell, and replacement text.
        ///
        /// # Returns
        /// Durable candidate metadata for shared review and apply.
        ///
        /// # Errors
        /// Returns authorization, stale-base, read-only fidelity, or candidate errors.
        fn propose_document_table_cell_edit(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::DocumentTableCellEditQuery,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            self.service.propose_document_table_cell_edit(
                &context.clone().with_snapshot(request.snapshot),
                request.project,
                request.cell,
                &request.replacement,
            )
        }

        /// Creates a spreadsheet edit candidate through the authorized application service.
        ///
        /// # Parameters
        /// - `context`: Caller authorization context.
        /// - `request`: Exact project, snapshot, worksheet, coordinate, and value.
        ///
        /// # Returns
        /// Candidate identity and current validation state.
        ///
        /// # Errors
        /// Returns authorization, stale-base, adapter, or candidate errors.
        fn propose_spreadsheet_edit(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellEditQuery,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            self.service.propose_spreadsheet_cell_edit(
                &context.clone().with_snapshot(request.snapshot),
                request.project,
                request.worksheet,
                request.row,
                request.column,
                &request.value,
            )
        }

        /// Inspects one exact worksheet coordinate through the shared application API.
        ///
        /// # Parameters
        /// - `context`: Caller authorization context.
        /// - `request`: Exact project, snapshot, worksheet, row, and column.
        ///
        /// # Returns
        /// Typed cell details normalized to the protocol response shape.
        ///
        /// # Errors
        /// Returns authorization, request, storage, or semantic decoding failures.
        fn inspect_spreadsheet_cell(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellQuery,
        ) -> Result<scriberig_protocol::SpreadsheetCellInspectResponse, ApplicationError> {
            scriberig_app::QueryService::inspect_spreadsheet_cell(&self.service, context, request)
                .map(crate::view::spreadsheet::cell_inspection_from_application)
        }

        /// Reads a bounded worksheet page through the shared application query API.
        ///
        /// # Parameters
        /// - `context`: Caller authorization context.
        /// - `request`: Exact project, snapshot, worksheet, range, and page bounds.
        ///
        /// # Returns
        /// Typed cells and continuation metadata normalized to the protocol shape.
        ///
        /// # Errors
        /// Returns authorization, request, storage, or decoding failures.
        fn inspect_spreadsheet_cells(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellsQuery,
        ) -> Result<scriberig_protocol::SpreadsheetCellsResponse, ApplicationError> {
            scriberig_app::QueryService::read_spreadsheet_cells(&self.service, context, request)
                .map(crate::view::spreadsheet::from_application)
        }
    };
}
