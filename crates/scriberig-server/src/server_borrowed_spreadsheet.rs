//! Forwards spreadsheet application operations through borrowed services.

macro_rules! shared_borrowed_spreadsheet_methods {
    () => {
        shared_borrowed_spreadsheet_rename_method!();

        /// Delegates an exact-snapshot spreadsheet cell inspection through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized semantic worksheet identity.
        /// - `row`: One-based row coordinate.
        /// - `column`: One-based column coordinate.
        ///
        /// # Returns
        /// Typed cell data or an absent sparse blank coordinate.
        ///
        /// # Errors
        /// Returns the delegated authorization or semantic read error.
        fn inspect_spreadsheet_cell(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            worksheet: &str,
            row: u32,
            column: u32,
        ) -> Result<scriberig_app::SpreadsheetCellInspection, scriberig_app::ApplicationError> {
            (**self).inspect_spreadsheet_cell(context, project, worksheet, row, column)
        }

        /// Delegates snapshot-bound spreadsheet reads through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized worksheet identity.
        /// - `range`: Optional rectangular coordinate filter.
        /// - `offset`: Zero-based page offset.
        /// - `limit`: Maximum returned cell count.
        ///
        /// # Returns
        /// Bounded typed worksheet cell page.
        ///
        /// # Errors
        /// Returns the delegated service error.
        fn read_spreadsheet_cells(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            worksheet: &str,
            range: Option<scriberig_protocol::SpreadsheetRange>,
            offset: usize,
            limit: usize,
        ) -> Result<scriberig_app::SpreadsheetCells, scriberig_app::ApplicationError> {
            (**self).read_spreadsheet_cells(context, project, worksheet, range, offset, limit)
        }

        /// Delegates a snapshot-bound workbook summary through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with Inspect authority.
        /// - `project`: Serialized project identity.
        /// - `workbook`: Serialized semantic workbook identity.
        ///
        /// # Returns
        /// Workbook metadata with ordered worksheet summaries.
        ///
        /// # Errors
        /// Returns the delegated authorization or semantic read error.
        fn inspect_spreadsheet_workbook(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            workbook: &str,
        ) -> Result<scriberig_project::SpreadsheetWorkbookSummary, scriberig_app::ApplicationError>
        {
            (**self).inspect_spreadsheet_workbook(context, project, workbook)
        }

        /// Delegates snapshot-bound spreadsheet search through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted exact-snapshot request context.
        /// - `request`: Typed workbook search and page selection.
        ///
        /// # Returns
        /// Bounded semantic cell matches with stable identities.
        ///
        /// # Errors
        /// Returns the delegated authorization or semantic search error.
        fn find_spreadsheet_cells(
            &self,
            context: &scriberig_app::ApplicationContext,
            request: &scriberig_app::SpreadsheetCellSearchRequest,
        ) -> Result<scriberig_project::SpreadsheetCellSearch, scriberig_app::ApplicationError> {
            (**self).find_spreadsheet_cells(context, request)
        }

        /// Delegates exact-base spreadsheet candidate creation through the borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with scoped Propose authority.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized worksheet identity.
        /// - `row`: One-based target row.
        /// - `column`: One-based target column.
        /// - `value`: Typed authored spreadsheet value.
        ///
        /// # Returns
        /// Candidate metadata without source bytes.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate error.
        fn propose_spreadsheet_cell_edit(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            worksheet: &str,
            row: u32,
            column: u32,
            value: scriberig_protocol::SpreadsheetCellValue,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, scriberig_app::ApplicationError> {
            (**self).propose_spreadsheet_cell_edit(context, project, worksheet, row, column, value)
        }

        /// Delegates bounded spreadsheet range candidate creation through a borrowed service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with scoped Propose authority.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized worksheet identity.
        /// - `edits`: Typed updates for distinct worksheet coordinates.
        ///
        /// # Returns
        /// Candidate metadata without source bytes.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate error.
        fn propose_spreadsheet_range_edit(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            worksheet: &str,
            edits: Vec<scriberig_protocol::SpreadsheetCellEdit>,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, scriberig_app::ApplicationError> {
            (**self).propose_spreadsheet_range_edit(context, project, worksheet, edits)
        }
    };
}
