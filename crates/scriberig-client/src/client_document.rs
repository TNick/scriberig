//! Provides semantic document operations over the protocol client.

use scriberig_protocol::{
    DocumentInspectRequest, DocumentInspectResponse, DocumentParagraphEditRequest,
    DocumentParagraphEditResponse, DocumentParagraphFindRequest, DocumentParagraphFindResponse,
    DocumentParagraphInsertRequest, DocumentParagraphInsertResponse, DocumentTableCellEditRequest,
    Request, Response,
};

use super::Client;
use crate::error::ClientError;

impl<T: scriberig_protocol::ProtocolTransport> Client<T> {
    /// Proposes an exact-base table-cell edit through its stable semantic handle.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable base snapshot.
    /// - `cell`: Stable table-cell identity.
    /// - `replacement`: Replacement cell text.
    ///
    /// # Returns
    /// Candidate handle for shared validation and review.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn propose_document_table_cell_edit(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        cell: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Result<DocumentParagraphEditResponse, ClientError> {
        match self
            .send(Request::DocumentTableCellEdit(
                DocumentTableCellEditRequest {
                    project: project.into(),
                    snapshot: snapshot.into(),
                    cell: cell.into(),
                    replacement: replacement.into(),
                },
            ))?
            .response
        {
            Response::DocumentTableCellEdit(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes an exact-base paragraph append to one semantic section.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable base snapshot.
    /// - `section`: Stable semantic section identity.
    /// - `text`: Authored text for the appended paragraph.
    ///
    /// # Returns
    /// Candidate handle for shared validation and review.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn propose_document_paragraph_insert(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        section: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<DocumentParagraphInsertResponse, ClientError> {
        match self
            .send(Request::DocumentParagraphInsert(
                DocumentParagraphInsertRequest {
                    project: project.into(),
                    snapshot: snapshot.into(),
                    section: section.into(),
                    text: text.into(),
                },
            ))?
            .response
        {
            Response::DocumentParagraphInsert(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Finds paragraph text at an exact document snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `document`: Stable semantic document root identity.
    /// - `query`: Non-empty case-insensitive text substring.
    /// - `offset`: Zero-based match offset.
    /// - `limit`: Maximum of one hundred matches.
    ///
    /// # Returns
    /// Bounded matching paragraph identities and authored text.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn find_document_paragraphs(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        document: impl Into<String>,
        query: impl Into<String>,
        offset: usize,
        limit: usize,
    ) -> Result<DocumentParagraphFindResponse, ClientError> {
        match self
            .send(Request::DocumentParagraphFind(
                DocumentParagraphFindRequest {
                    project: project.into(),
                    snapshot: snapshot.into(),
                    document: document.into(),
                    query: query.into(),
                    offset,
                    limit,
                },
            ))?
            .response
        {
            Response::DocumentParagraphFind(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads a typed document tree at an exact immutable snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `document`: Stable semantic document root identity.
    ///
    /// # Returns
    /// Ordered sections, paragraphs, and tables with stable semantic IDs.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn inspect_document(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        document: impl Into<String>,
    ) -> Result<DocumentInspectResponse, ClientError> {
        match self
            .send(Request::DocumentInspect(DocumentInspectRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                document: document.into(),
            }))?
            .response
        {
            Response::DocumentInspect(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes a semantic document paragraph edit as a reviewable candidate.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable base snapshot.
    /// - `paragraph`: Stable semantic paragraph identity.
    /// - `replacement`: Replacement paragraph text.
    ///
    /// # Returns
    /// Candidate handle for shared validation and review.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn propose_document_paragraph_edit(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        paragraph: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Result<DocumentParagraphEditResponse, ClientError> {
        match self
            .send(Request::DocumentParagraphEdit(
                DocumentParagraphEditRequest {
                    project: project.into(),
                    snapshot: snapshot.into(),
                    paragraph: paragraph.into(),
                    replacement: replacement.into(),
                },
            ))?
            .response
        {
            Response::DocumentParagraphEdit(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
