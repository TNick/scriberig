//! Forwards worksheet rename requests through borrowed application services.

macro_rules! shared_borrowed_spreadsheet_rename_method {
    () => {
        /// Delegates a worksheet rename through the borrowed application service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with scoped Propose authority.
        /// - `project`: Serialized project identity.
        /// - `worksheet`: Serialized worksheet identity.
        /// - `new_name`: Requested authored worksheet name.
        ///
        /// # Returns
        /// Candidate metadata without workbook bytes.
        ///
        /// # Errors
        /// Returns the delegated authorization or candidate error.
        fn propose_spreadsheet_rename(
            &self,
            context: &scriberig_app::ApplicationContext,
            project: &str,
            worksheet: &str,
            new_name: &str,
        ) -> Result<scriberig_app::SpreadsheetEditCandidate, scriberig_app::ApplicationError> {
            (**self).propose_spreadsheet_rename(context, project, worksheet, new_name)
        }
    };
}
