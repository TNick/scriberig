//! Provides typed client operations for shared projection style semantics.

use scriberig_protocol::{
    ProjectionStyleResolveRequest, ProjectionStyleResolveResponse, ProjectionStyleSchemaRequest,
    ProjectionStyleSchemaResponse, ProjectionStyleSettingRequest, ProjectionStyleSettingResponse,
    Request, Response,
};

use super::{Client, ProtocolTransport};
use crate::error::ClientError;

impl<T: ProtocolTransport> Client<T> {
    /// Reads the adapter-owned style schema for a projection kind.
    ///
    /// - `projection_kind`: Stable projection kind such as `javascript`.
    ///
    /// The schema when the domain publishes one.
    ///
    /// # Returns
    /// The optional schema response.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn projection_style_schema(
        &self,
        projection_kind: impl Into<String>,
    ) -> Result<Option<ProjectionStyleSchemaResponse>, ClientError> {
        match self
            .send(Request::ProjectionStyleSchema(
                ProjectionStyleSchemaRequest {
                    projection_kind: projection_kind.into(),
                },
            ))?
            .response
        {
            Response::ProjectionStyleSchema(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Resolves effective style, provenance, and fidelity from shared semantics.
    ///
    /// - `request`: Exact style purpose and project/workspace/view scope.
    ///
    /// The resolved style response.
    ///
    /// # Returns
    /// Exact style bytes with contributing sources and fidelity.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn resolve_projection_style(
        &self,
        request: ProjectionStyleResolveRequest,
    ) -> Result<ProjectionStyleResolveResponse, ClientError> {
        match self
            .send(Request::ProjectionStyleResolve(request))?
            .response
        {
            Response::ProjectionStyleResolve(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Sets or clears a projection style setting through the application service.
    ///
    /// - `request`: Explicit scope, kind, expected revision, and optional style bytes.
    ///
    /// The authoritative setting result.
    ///
    /// # Returns
    /// Stored style bytes and revision, or empty fields after a clear.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn set_projection_style(
        &self,
        request: ProjectionStyleSettingRequest,
    ) -> Result<ProjectionStyleSettingResponse, ClientError> {
        match self
            .send(Request::ProjectionStyleSetting(request))?
            .response
        {
            Response::ProjectionStyleSetting(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
