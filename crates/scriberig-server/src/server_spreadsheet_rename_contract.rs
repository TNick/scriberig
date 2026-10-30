//! Defines the optional worksheet rename operation for server application adapters.

macro_rules! spreadsheet_rename_contract {
    () => {
        /// Proposes an exact-base worksheet rename through shared authorization.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, snapshot, and scoped grants.
        /// - `_project`: Serialized stable project identity.
        /// - `_worksheet`: Serialized stable worksheet identity.
        /// - `_new_name`: Requested authored worksheet name.
        ///
        /// # Returns
        /// Durable candidate handle for shared validation and review.
        ///
        /// # Errors
        /// Returns unsupported operation unless worksheet renames are implemented.
        fn propose_spreadsheet_rename(
            &self,
            _context: &ApplicationContext,
            _project: &str,
            _worksheet: &str,
            _new_name: &str,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "spreadsheet.sheet.rename".into(),
            })
        }
    };
}
