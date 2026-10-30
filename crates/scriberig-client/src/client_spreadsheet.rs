//! Provides typed spreadsheet and artifact inspection methods on the semantic client.

use scriberig_protocol::{
    ArtifactInspectRequest, ArtifactInspectResponse, ArtifactMembersRequest,
    ArtifactMembersResponse, Request, Response, SpreadsheetCellFindRequest,
    SpreadsheetCellFindResponse, SpreadsheetCellInspectRequest, SpreadsheetCellInspectResponse,
    SpreadsheetCellValue, SpreadsheetCellsRequest, SpreadsheetCellsResponse,
    SpreadsheetEditRequest, SpreadsheetEditResponse, SpreadsheetRange, SpreadsheetRangeEditRequest,
    SpreadsheetRenameRequest, SpreadsheetWorkbookRequest, SpreadsheetWorkbookResponse,
};

use crate::{client::Client, error::ClientError};

impl<T: scriberig_protocol::ProtocolTransport> Client<T> {
    /// Proposes multiple spreadsheet cell updates as one reviewable candidate.
    ///
    /// # Parameters
    /// - `request`: Exact project snapshot and bounded typed cell updates.
    ///
    /// # Returns
    /// Candidate handle for shared validation and review.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn propose_spreadsheet_range_edit(
        &self,
        request: SpreadsheetRangeEditRequest,
    ) -> Result<SpreadsheetEditResponse, ClientError> {
        match self.send(Request::SpreadsheetRangeEdit(request))?.response {
            Response::SpreadsheetRangeEdit(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Finds authored cell values at an exact workbook snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `workbook`: Stable semantic workbook identity.
    /// - `query`: Non-empty authored-value substring.
    /// - `offset`: Zero-based match offset.
    /// - `limit`: Maximum of one hundred matches.
    ///
    /// # Returns
    /// Bounded cell matches with stable worksheet and cell identities.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn find_spreadsheet_cells(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        workbook: impl Into<String>,
        query: impl Into<String>,
        offset: usize,
        limit: usize,
    ) -> Result<SpreadsheetCellFindResponse, ClientError> {
        match self
            .send(Request::SpreadsheetCellFind(SpreadsheetCellFindRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                workbook: workbook.into(),
                query: query.into(),
                offset,
                limit,
            }))?
            .response
        {
            Response::SpreadsheetCellFind(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Retrieves one bounded page of exact artifact-member evidence.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `artifact`: Stable semantic artifact identity.
    /// - `offset`: Zero-based member offset.
    /// - `limit`: Maximum number of member descriptors, from one to five hundred.
    ///
    /// # Returns
    /// Physical and container-part evidence with continuation metadata.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn inspect_artifact_members(
        &self,
        project: impl Into<String>,
        artifact: impl Into<String>,
        offset: usize,
        limit: usize,
    ) -> Result<ArtifactMembersResponse, ClientError> {
        match self
            .send(Request::ArtifactMembers(ArtifactMembersRequest {
                project: project.into(),
                artifact: artifact.into(),
                offset,
                limit,
            }))?
            .response
        {
            Response::ArtifactMembers(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads one semantic cell coordinate at an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `row`: One-based row coordinate.
    /// - `column`: One-based column coordinate.
    ///
    /// # Returns
    /// Typed cell data or an absent sparse blank coordinate.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn inspect_spreadsheet_cell(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        worksheet: impl Into<String>,
        row: u32,
        column: u32,
    ) -> Result<SpreadsheetCellInspectResponse, ClientError> {
        match self
            .send(Request::SpreadsheetCellInspect(
                SpreadsheetCellInspectRequest {
                    project: project.into(),
                    snapshot: snapshot.into(),
                    worksheet: worksheet.into(),
                    row,
                    column,
                },
            ))?
            .response
        {
            Response::SpreadsheetCellInspect(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Retrieves a bounded page of project artifact summaries.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `offset`: Zero-based artifact offset.
    /// - `limit`: Maximum number of summaries, from one to one hundred.
    ///
    /// # Returns
    /// Bounded artifact summary response.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn inspect_artifacts(
        &self,
        project: impl Into<String>,
        offset: usize,
        limit: usize,
    ) -> Result<ArtifactInspectResponse, ClientError> {
        match self
            .send(Request::ArtifactInspect(ArtifactInspectRequest {
                project: project.into(),
                offset,
                limit,
            }))?
            .response
        {
            Response::ArtifactInspect(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads a workbook and ordered worksheet summaries at an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `workbook`: Stable semantic workbook identity.
    ///
    /// # Returns
    /// Typed workbook metadata with ordered worksheet names and used ranges.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn inspect_spreadsheet_workbook(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        workbook: impl Into<String>,
    ) -> Result<SpreadsheetWorkbookResponse, ClientError> {
        match self
            .send(Request::SpreadsheetWorkbook(SpreadsheetWorkbookRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                workbook: workbook.into(),
            }))?
            .response
        {
            Response::SpreadsheetWorkbook(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes a semantic spreadsheet cell edit as a reviewable candidate.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable base snapshot.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `row`: One-based row coordinate.
    /// - `column`: One-based column coordinate.
    /// - `value`: Authored value or formula.
    ///
    /// # Returns
    /// Candidate handle for shared validation and review.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn propose_spreadsheet_cell_edit(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        worksheet: impl Into<String>,
        row: u32,
        column: u32,
        value: SpreadsheetCellValue,
    ) -> Result<SpreadsheetEditResponse, ClientError> {
        match self
            .send(Request::SpreadsheetEdit(SpreadsheetEditRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                worksheet: worksheet.into(),
                row,
                column,
                value,
            }))?
            .response
        {
            Response::SpreadsheetEdit(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes a semantic worksheet rename as a reviewable candidate.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable base snapshot.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `new_name`: Requested authored worksheet name.
    ///
    /// # Returns
    /// Candidate handle for shared validation and review.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn propose_spreadsheet_rename(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        worksheet: impl Into<String>,
        new_name: impl Into<String>,
    ) -> Result<SpreadsheetEditResponse, ClientError> {
        match self
            .send(Request::SpreadsheetRename(SpreadsheetRenameRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                worksheet: worksheet.into(),
                new_name: new_name.into(),
            }))?
            .response
        {
            Response::SpreadsheetRename(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads a bounded sparse spreadsheet page at an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `worksheet`: Stable worksheet identity.
    /// - `range`: Optional inclusive one-based rectangular bounds.
    /// - `offset`: Zero-based page offset.
    /// - `limit`: Maximum cell count, from one to five hundred.
    ///
    /// # Returns
    /// Snapshot-bound typed cell page and continuation metadata.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn read_spreadsheet_cells(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        worksheet: impl Into<String>,
        range: Option<SpreadsheetRange>,
        offset: usize,
        limit: usize,
    ) -> Result<SpreadsheetCellsResponse, ClientError> {
        match self
            .send(Request::SpreadsheetCells(SpreadsheetCellsRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                worksheet: worksheet.into(),
                range,
                offset,
                limit,
            }))?
            .response
        {
            Response::SpreadsheetCells(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
