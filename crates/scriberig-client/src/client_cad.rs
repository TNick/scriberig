//! Provides semantic CAD line edit operations over the protocol client.

use scriberig_protocol::{
    CadBoundsQueryRequest, CadBoundsQueryResponse, CadCoordinate, CadDrawingInspectRequest,
    CadDrawingInspectResponse, CadEntitiesRequest, CadEntitiesResponse, CadLayerChangeRequest,
    CadLayerChangeResponse, CadLineEditRequest, CadLineEditResponse, CadTextFindRequest,
    CadTextFindResponse, Request, Response,
};

use super::Client;
use crate::error::ClientError;

impl<T: scriberig_protocol::ProtocolTransport> Client<T> {
    /// Reads entities intersecting exact drawing-space bounds.
    ///
    /// # Parameters
    /// - `request`: Typed exact-snapshot CAD bounds query.
    ///
    /// # Returns
    /// Bounded semantic entities and extent completeness metadata.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response type.
    pub fn cad_entities_intersecting_bounds(
        &self,
        request: CadBoundsQueryRequest,
    ) -> Result<CadBoundsQueryResponse, ClientError> {
        match self.send(Request::CadBoundsQuery(request))?.response {
            Response::CadBoundsQuery(page) => Ok(page),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads an exact-snapshot semantic CAD drawing summary and layers.
    ///
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `drawing`: Stable semantic drawing identity.
    ///
    /// Typed CAD summary and semantic layers.
    ///
    /// # Returns
    /// The requested drawing summary.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn inspect_cad_drawing(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        drawing: impl Into<String>,
    ) -> Result<CadDrawingInspectResponse, ClientError> {
        match self
            .send(Request::CadDrawingInspect(CadDrawingInspectRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                drawing: drawing.into(),
            }))?
            .response
        {
            Response::CadDrawingInspect(view) => Ok(view),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads one bounded page of semantic CAD entities.
    ///
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `drawing`: Stable semantic drawing identity.
    /// - `offset`: Zero-based source-order offset.
    /// - `limit`: Requested page size from one to five hundred.
    ///
    /// Typed semantic entity page.
    ///
    /// # Returns
    /// The bounded entity page and continuation metadata.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response.
    pub fn cad_entities(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        drawing: impl Into<String>,
        offset: usize,
        limit: usize,
    ) -> Result<CadEntitiesResponse, ClientError> {
        match self
            .send(Request::CadEntities(CadEntitiesRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                drawing: drawing.into(),
                offset,
                limit,
            }))?
            .response
        {
            Response::CadEntities(page) => Ok(page),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Finds authored CAD annotation text in one exact-snapshot drawing.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable snapshot identity.
    /// - `drawing`: Stable semantic drawing identity.
    /// - `query`: Case-insensitive text substring.
    /// - `offset`: Zero-based matching result offset.
    /// - `limit`: Requested page size from one to one hundred.
    ///
    /// # Returns
    /// Bounded text matches with stable entity identities and exact geometry.
    ///
    /// # Errors
    /// Returns a structured server error or unexpected response type.
    pub fn find_cad_text(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        drawing: impl Into<String>,
        query: impl Into<String>,
        offset: usize,
        limit: usize,
    ) -> Result<CadTextFindResponse, ClientError> {
        match self
            .send(Request::CadTextFind(CadTextFindRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                drawing: drawing.into(),
                query: query.into(),
                offset,
                limit,
            }))?
            .response
        {
            Response::CadTextFind(matches) => Ok(matches),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes a line geometry edit as a reviewable exact-base candidate.
    ///
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable base snapshot.
    /// - `entity`: Stable semantic CAD line identity.
    /// - `start`: Exact decimal start coordinate.
    /// - `end`: Exact decimal end coordinate.
    ///
    /// # Returns
    /// Candidate handle for shared validation and review.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response type.
    pub fn propose_cad_line_edit(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        entity: impl Into<String>,
        start: CadCoordinate,
        end: CadCoordinate,
    ) -> Result<CadLineEditResponse, ClientError> {
        match self
            .send(Request::CadLineEdit(CadLineEditRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                entity: entity.into(),
                start,
                end,
            }))?
            .response
        {
            Response::CadLineEdit(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Proposes moving one CAD entity to an existing layer as a reviewable candidate.
    ///
    /// # Parameters
    /// - `project`: Stable project identity.
    /// - `snapshot`: Exact immutable base snapshot.
    /// - `entity`: Stable semantic CAD entity identity.
    /// - `destination_layer`: Stable semantic layer identity in the same drawing.
    ///
    /// # Returns
    /// Candidate handle for shared validation and review.
    ///
    /// # Errors
    /// Returns a structured server error or an unexpected response type.
    pub fn propose_cad_layer_change(
        &self,
        project: impl Into<String>,
        snapshot: impl Into<String>,
        entity: impl Into<String>,
        destination_layer: impl Into<String>,
    ) -> Result<CadLayerChangeResponse, ClientError> {
        match self
            .send(Request::CadLayerChange(CadLayerChangeRequest {
                project: project.into(),
                snapshot: snapshot.into(),
                entity: entity.into(),
                destination_layer: destination_layer.into(),
            }))?
            .response
        {
            Response::CadLayerChange(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
