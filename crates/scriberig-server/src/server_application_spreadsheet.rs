//! Adds typed spreadsheet candidate operations to the application adapter.

macro_rules! spreadsheet_methods {
    () => {
        /// Inspects one cell coordinate through the shared application boundary.
        ///
        /// # Parameters
        /// - `context`: Trusted request context containing the exact snapshot and grant.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized semantic worksheet identity.
        /// - `row`: One-based target row.
        /// - `column`: One-based target column.
        ///
        /// # Returns
        /// Typed semantic cell or an absent sparse blank coordinate.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project read errors.
        fn inspect_spreadsheet_cell(
            &self,
            context: &ApplicationContext,
            project: &str,
            worksheet: &str,
            row: u32,
            column: u32,
        ) -> Result<scriberig_app::SpreadsheetCellInspection, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "spreadsheet.cell.inspect".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let worksheet =
                worksheet
                    .parse()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "spreadsheet.cell.inspect".into(),
                        message: format!("invalid worksheet identity: {error}"),
                    })?;
            scriberig_app::ApplicationService::inspect_spreadsheet_cell(
                self, context, project, worksheet, row, column,
            )
        }

        /// Reads a workbook and ordered worksheet summaries through shared authority.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with exact snapshot and Inspect grant.
        /// - `project`: Serialized project identity.
        /// - `workbook`: Serialized semantic workbook identity.
        ///
        /// # Returns
        /// Workbook metadata with ordered worksheet names and used ranges.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or semantic read errors.
        fn inspect_spreadsheet_workbook(
            &self,
            context: &ApplicationContext,
            project: &str,
            workbook: &str,
        ) -> Result<scriberig_project::SpreadsheetWorkbookSummary, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "spreadsheet.workbook.inspect".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let workbook = workbook
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "spreadsheet.workbook.inspect".into(),
                    message: format!("invalid workbook identity: {error}"),
                })?;
            scriberig_app::ApplicationService::inspect_spreadsheet_workbook(
                self, context, project, workbook,
            )
        }

        /// Finds authored workbook cell values through the shared application boundary.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with exact snapshot and Inspect grant.
        /// - `request`: Typed workbook search and page selection.
        ///
        /// # Returns
        /// Bounded semantic cell matches with stable identities.
        ///
        /// # Errors
        /// Returns authorization, availability, or project search errors.
        fn find_spreadsheet_cells(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::SpreadsheetCellSearchRequest,
        ) -> Result<scriberig_project::SpreadsheetCellSearch, ApplicationError> {
            scriberig_app::ApplicationService::find_spreadsheet_cells(self, context, request)
        }

        /// Creates a spreadsheet source candidate through the shared project service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context containing exact snapshot and grants.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized worksheet identity.
        /// - `row`: One-based target row.
        /// - `column`: One-based target column.
        /// - `value`: Typed authored cell value or formula.
        ///
        /// # Returns
        /// Candidate metadata for shared review and validation.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project candidate errors.
        fn propose_spreadsheet_cell_edit(
            &self,
            context: &ApplicationContext,
            project: &str,
            worksheet: &str,
            row: u32,
            column: u32,
            value: scriberig_protocol::SpreadsheetCellValue,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "spreadsheet.cell.propose".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let worksheet =
                worksheet
                    .parse()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "spreadsheet.cell.propose".into(),
                        message: format!("invalid worksheet identity: {error}"),
                    })?;
            let value = super::server_spreadsheet::cell_value(value);
            scriberig_app::ApplicationService::propose_spreadsheet_cell_edit(
                self, context, project, worksheet, row, column, &value,
            )
        }

        /// Creates a bounded multi-cell candidate through the shared project service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context containing exact snapshot and grants.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized worksheet identity.
        /// - `edits`: One through five hundred distinct typed cell updates.
        ///
        /// # Returns
        /// Candidate metadata for shared review and validation.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project candidate errors.
        fn propose_spreadsheet_range_edit(
            &self,
            context: &ApplicationContext,
            project: &str,
            worksheet: &str,
            edits: Vec<scriberig_protocol::SpreadsheetCellEdit>,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "spreadsheet.range.propose".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let worksheet =
                worksheet
                    .parse()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "spreadsheet.range.propose".into(),
                        message: format!("invalid worksheet identity: {error}"),
                    })?;
            let edits = edits
                .into_iter()
                .map(|edit| scriberig_app::SpreadsheetCellEdit {
                    row: edit.row,
                    column: edit.column,
                    value: super::server_spreadsheet::cell_value(edit.value),
                })
                .collect::<Vec<_>>();
            scriberig_app::ApplicationService::propose_spreadsheet_range_edit(
                self, context, project, worksheet, &edits,
            )
        }

        /// Creates a snapshot-bound worksheet rename candidate.
        ///
        /// # Parameters
        /// - `context`: Trusted request context containing exact snapshot and grants.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized semantic worksheet identity.
        /// - `new_name`: Requested authored worksheet name.
        ///
        /// # Returns
        /// Candidate metadata for shared review and validation.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project candidate errors.
        fn propose_spreadsheet_rename(
            &self,
            context: &ApplicationContext,
            project: &str,
            worksheet: &str,
            new_name: &str,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "spreadsheet.sheet.rename".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let worksheet =
                worksheet
                    .parse()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "spreadsheet.sheet.rename".into(),
                        message: format!("invalid worksheet identity: {error}"),
                    })?;
            scriberig_app::ApplicationService::propose_spreadsheet_sheet_rename(
                self, context, project, worksheet, new_name,
            )
        }
    };
}
