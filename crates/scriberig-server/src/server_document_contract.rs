//! Adds semantic document edit operations to the server application contract.

macro_rules! document_edit_contract {
    () => {
        /// Reads a bounded document tree from one exact snapshot.
        ///
        /// - `_context`: Trusted principal, snapshot, and scoped Inspect grants.
        /// - `_project`: Serialized project identity.
        /// - `_snapshot`: Serialized exact snapshot identity.
        /// - `_document`: Serialized semantic document identity.
        ///
        /// # Returns
        /// Typed semantic document tree.
        ///
        /// Returns authorization, identity, snapshot, or project read errors.
        ///
        /// # Errors
        /// Returns authorization, identity, snapshot, or project read errors.
        fn inspect_document(
            &self,
            _context: &scriberig_app::ApplicationContext,
            _project: &str,
            _snapshot: &str,
            _document: &str,
        ) -> Result<scriberig_app::DocumentTreeView, scriberig_app::ApplicationError> {
            Err(scriberig_app::ApplicationError::UnsupportedOperation {
                operation: "document.inspect".into(),
            })
        }

        /// Proposes an exact-base semantic paragraph insertion.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, snapshot, and scoped grants.
        /// - `_project`: Serialized project identity.
        /// - `_section`: Serialized section identity.
        /// - `_text`: New authored paragraph text.
        ///
        /// # Returns
        /// Candidate handle for shared validation and review.
        ///
        /// # Errors
        /// Returns authorization, identity, unsupported-structure, or candidate errors.
        fn propose_document_paragraph_insert(
            &self,
            _context: &scriberig_app::ApplicationContext,
            _project: &str,
            _section: &str,
            _text: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, scriberig_app::ApplicationError> {
            Err(scriberig_app::ApplicationError::UnsupportedOperation {
                operation: "document.paragraph.insert".into(),
            })
        }

        /// Proposes an exact-base semantic paragraph edit.
        /// Proposes an exact-base semantic paragraph edit.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, snapshot, and scoped grants.
        /// - `_project`: Serialized project identity.
        /// - `_paragraph`: Serialized paragraph identity.
        /// - `_replacement`: New paragraph text.
        ///
        /// # Returns
        /// Candidate handle for shared validation and review.
        ///
        /// # Errors
        /// Returns authorization, identity, unsupported-structure, or candidate errors.
        fn propose_document_paragraph_edit(
            &self,
            _context: &scriberig_app::ApplicationContext,
            _project: &str,
            _paragraph: &str,
            _replacement: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, scriberig_app::ApplicationError> {
            Err(scriberig_app::ApplicationError::UnsupportedOperation {
                operation: "document.paragraph.propose".into(),
            })
        }

        /// Proposes an exact-base semantic table-cell edit.
        ///
        /// # Parameters
        /// - `_context`: Trusted principal, snapshot, and scoped grants.
        /// - `_project`: Serialized project identity.
        /// - `_cell`: Serialized semantic table-cell identity.
        /// - `_replacement`: New cell text.
        ///
        /// # Returns
        /// Candidate handle for shared validation and review.
        ///
        /// # Errors
        /// Returns authorization, identity, unsupported-structure, or candidate errors.
        fn propose_document_table_cell_edit(
            &self,
            _context: &scriberig_app::ApplicationContext,
            _project: &str,
            _cell: &str,
            _replacement: &str,
        ) -> Result<scriberig_app::DocumentParagraphCandidate, scriberig_app::ApplicationError> {
            Err(scriberig_app::ApplicationError::UnsupportedOperation {
                operation: "document.table_cell.propose".into(),
            })
        }
    };
}
