//! Adapts semantic document edits to the shared application service.

macro_rules! document_methods {
    () => {
        /// Finds paragraph text through the shared application boundary.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with exact snapshot and grant.
        /// - `request`: Typed project, snapshot, document, query, and page selection.
        ///
        /// # Returns
        /// Bounded ordered paragraph matches.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project search errors.
        fn find_document_paragraphs(
            &self,
            context: &ApplicationContext,
            request: &scriberig_app::DocumentParagraphSearchRequest,
        ) -> Result<scriberig_app::DocumentParagraphSearch, ApplicationError> {
            scriberig_app::ApplicationService::find_document_paragraphs(self, context, request)
        }

        /// Reads a snapshot-bound rich-document tree through the shared application.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with exact snapshot and Inspect grant.
        /// - `project`: Serialized project identity.
        /// - `snapshot`: Serialized exact snapshot identity.
        /// - `document`: Serialized semantic document identity.
        ///
        /// # Returns
        /// Typed ordered semantic document tree.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project read errors.
        fn inspect_document(
            &self,
            context: &ApplicationContext,
            project: &str,
            snapshot: &str,
            document: &str,
        ) -> Result<scriberig_app::DocumentTreeView, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "document.inspect".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let snapshot = snapshot
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "document.inspect".into(),
                    message: format!("invalid snapshot identity: {error}"),
                })?;
            let document = document
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "document.inspect".into(),
                    message: format!("invalid document identity: {error}"),
                })?;
            scriberig_app::ApplicationService::inspect_document(
                self, context, project, snapshot, document,
            )
        }

        /// Proposes an exact-base semantic paragraph append.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with exact snapshot and grants.
        /// - `project`: Serialized project identity.
        /// - `section`: Serialized semantic section identity.
        /// - `text`: New authored paragraph text.
        ///
        /// # Returns
        /// Candidate metadata for shared validation and review.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project candidate errors.
        fn propose_document_paragraph_insert(
            &self,
            context: &ApplicationContext,
            project: &str,
            section: &str,
            text: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "document.paragraph.insert".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let section = section
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "document.paragraph.insert".into(),
                    message: format!("invalid section identity: {error}"),
                })?;
            scriberig_app::ApplicationService::propose_document_paragraph_insert(
                self, context, project, section, text,
            )
        }

        /// Proposes a DOCX paragraph candidate through the shared project service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context with exact snapshot and grants.
        /// - `project`: Serialized project identity.
        /// - `paragraph`: Serialized semantic paragraph identity.
        /// - `replacement`: New authored paragraph text.
        ///
        /// # Returns
        /// Candidate metadata for shared validation and review.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project candidate errors.
        fn propose_document_paragraph_edit(
            &self,
            context: &ApplicationContext,
            project: &str,
            paragraph: &str,
            replacement: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "document.paragraph.propose".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let paragraph =
                paragraph
                    .parse()
                    .map_err(|error| ApplicationError::InvalidRequest {
                        operation: "document.paragraph.propose".into(),
                        message: format!("invalid paragraph identity: {error}"),
                    })?;
            scriberig_app::ApplicationService::propose_document_paragraph_edit(
                self,
                context,
                project,
                paragraph,
                replacement,
            )
        }

        /// Creates a semantic table-cell candidate through the shared project service.
        ///
        /// # Parameters
        /// - `context`: Trusted request context containing exact snapshot and grants.
        /// - `project`: Serialized project identity.
        /// - `cell`: Serialized table-cell identity.
        /// - `replacement`: New authored cell text.
        ///
        /// # Returns
        /// Candidate metadata for shared validation and review.
        ///
        /// # Errors
        /// Returns malformed identities, authorization, or project candidate errors.
        fn propose_document_table_cell_edit(
            &self,
            context: &ApplicationContext,
            project: &str,
            cell: &str,
            replacement: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, ApplicationError> {
            let project = project
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "document.table_cell.propose".into(),
                    message: format!("invalid project identity: {error}"),
                })?;
            let cell = cell
                .parse()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "document.table_cell.propose".into(),
                    message: format!("invalid table-cell identity: {error}"),
                })?;
            scriberig_app::ApplicationService::propose_document_table_cell_edit(
                self,
                context,
                project,
                cell,
                replacement,
            )
        }
    };
}
