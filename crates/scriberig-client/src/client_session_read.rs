//! Provides the compact root-session read on the reusable semantic client.

use scriberig_protocol::{
    ProtocolTransport, Request, Response, SessionReadRequest, SessionReadResponse, SessionReference,
};

use super::Client;
use crate::error::ClientError;

impl<T: ProtocolTransport> Client<T> {
    /// Reads one compact durable session tree.
    ///
    /// # Parameters
    /// - `session`: Stable session identity.
    ///
    /// # Returns
    /// Provider-neutral session and branch activity summaries.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn read_session(
        &self,
        session: impl Into<String>,
    ) -> Result<SessionReadResponse, ClientError> {
        match self
            .send(Request::SessionRead(SessionReadRequest {
                session: SessionReference { id: session.into() },
                branch: None,
            }))?
            .response
        {
            Response::Session(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
