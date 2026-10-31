//! Declares default document, spreadsheet, and artifact methods for the TUI service contract.

macro_rules! tui_document_artifact_methods {
    () => {
        /// Reads an exact-snapshot semantic document tree through a shared service.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, and document identity.
        ///
        /// # Returns
        /// Ordered semantic sections, paragraphs, and tables.
        ///
        /// # Errors
        /// Returns unsupported-operation, authorization, stale-snapshot, or transport errors.
        fn inspect_document(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::DocumentTreeQuery,
        ) -> Result<scriberig_protocol::DocumentInspectResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "document inspection".to_owned(),
            })
        }

        /// Creates a paragraph replacement candidate through shared document operations.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, base snapshot, paragraph ID, and replacement text.
        ///
        /// # Returns
        /// Durable candidate metadata for the common review/apply flow.
        ///
        /// # Errors
        /// Returns unsupported-operation, authorization, stale-base, or transport errors.
        fn propose_document_paragraph_edit(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::DocumentParagraphEditQuery,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "document paragraph edit candidate".to_owned(),
            })
        }

        /// Creates a table-cell replacement candidate through shared document operations.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, base snapshot, cell ID, and replacement text.
        ///
        /// # Returns
        /// Durable candidate metadata for the common review/apply flow.
        ///
        /// # Errors
        /// Returns unsupported-operation, authorization, stale-base, or transport errors.
        fn propose_document_table_cell_edit(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::DocumentTableCellEditQuery,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "document table-cell edit candidate".to_owned(),
            })
        }

        /// Reads a bounded worksheet page through the shared local or remote API.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, worksheet, range, and page bounds.
        ///
        /// # Returns
        /// Typed cells, fidelity, and continuation metadata at the requested snapshot.
        ///
        /// # Errors
        /// Returns unsupported-operation, authorization, request, transport, or decoding failures.
        fn inspect_spreadsheet_cells(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::SpreadsheetCellsQuery,
        ) -> Result<scriberig_protocol::SpreadsheetCellsResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet cell inspection".to_owned(),
            })
        }

        /// Inspects one exact worksheet coordinate through the shared local or remote API.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, worksheet, row, and column.
        ///
        /// # Returns
        /// Typed cell details, including an explicit sparse blank result.
        ///
        /// # Errors
        /// Returns unsupported-operation, authorization, request, transport, or decoding failures.
        fn inspect_spreadsheet_cell(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::SpreadsheetCellQuery,
        ) -> Result<scriberig_protocol::SpreadsheetCellInspectResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet cell inspection".to_owned(),
            })
        }

        /// Proposes one shared semantic cell edit as a durable reviewable candidate.
        ///
        /// # Parameters
        /// - `_context`: Caller authorization context.
        /// - `_request`: Exact project, snapshot, worksheet, coordinate, and typed value.
        ///
        /// # Returns
        /// Candidate identity and validation state for the shared change review flow.
        ///
        /// # Errors
        /// Returns unsupported-operation, authorization, request, transport, or candidate failures.
        fn propose_spreadsheet_edit(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_app::SpreadsheetCellEditQuery,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet cell edit candidate".to_owned(),
            })
        }

        /// Reads one bounded project artifact page through the trusted shared service.
        ///
        /// # Parameters
        /// - `_context`: Caller and scoped capability context.
        /// - `_project`: Stable project identity.
        /// - `_offset`: Zero-based artifact offset.
        /// - `_limit`: Maximum number of artifact rows.
        ///
        /// # Returns
        /// Terminal-safe artifact metadata with exact member digests and fidelity.
        ///
        /// # Errors
        /// Returns authorization, availability, or request failures.
        fn inspect_artifact_page(
            &self,
            _context: &ApplicationContext,
            _project: scriberig_core::ProjectId,
            _offset: usize,
            _limit: usize,
        ) -> Result<crate::view::artifacts::ArtifactPageModel, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "artifact inspection".to_owned(),
            })
        }

        /// Reads one bounded member page through the shared artifact Inspect operation.
        ///
        /// # Parameters
        /// - `_context`: Caller and scoped capability context.
        /// - `_project`: Stable project identity.
        /// - `_artifact`: Stable semantic artifact identity.
        /// - `_offset`: Zero-based member offset.
        /// - `_limit`: Maximum number of member descriptors.
        ///
        /// # Returns
        /// Terminal-safe physical and container-part evidence.
        ///
        /// # Errors
        /// Returns authorization, availability, or request failures.
        fn inspect_artifact_members(
            &self,
            _context: &ApplicationContext,
            _project: scriberig_core::ProjectId,
            _artifact: scriberig_core::EntityId,
            _offset: usize,
            _limit: usize,
        ) -> Result<crate::view::ArtifactMembersModel, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "artifact member inspection".to_owned(),
            })
        }
    };
}
