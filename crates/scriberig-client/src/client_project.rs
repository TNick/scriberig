//! Provides project onboarding methods for the semantic client.

use scriberig_protocol::{ProjectRequest, ProtocolTransport, Request, Response};

use crate::{Client, error::ClientError};

impl<T: ProtocolTransport> Client<T> {
    /// Executes one typed project onboarding operation.
    ///
    /// # Parameters
    /// - `operation`: Stable project operation such as `scan` or `import`.
    /// - `payload`: Operation-specific JSON payload.
    ///
    /// # Returns
    /// The presentation-neutral project operation result.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn project(
        &self,
        operation: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, ClientError> {
        match self
            .send(Request::Project(ProjectRequest {
                operation: operation.into(),
                payload,
            }))?
            .response
        {
            Response::Project(response) => Ok(response.result),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
