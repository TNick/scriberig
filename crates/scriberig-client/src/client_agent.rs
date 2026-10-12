//! Provides agent-operation methods for the protocol client.

use super::{
    AgentContinueRequest, AgentStartRequest, BranchReference, Client, ClientError,
    OperationStatusResponse, ProtocolTransport, Request, Response, SessionReference,
};

impl<T: ProtocolTransport> Client<T> {
    /// Submits a session agent command through the shared operation boundary.
    ///
    /// # Parameters
    /// - `session`: Stable target session identity.
    /// - `branch`: Optional branch receiving the command.
    /// - `prompt`: User-authored agent instruction.
    ///
    /// # Returns
    /// The accepted shared operation status.
    ///
    /// # Errors
    /// Returns a structured server or transport error.
    pub fn submit_agent_command(
        &self,
        session: impl Into<String>,
        branch: Option<impl Into<String>>,
        prompt: impl Into<String>,
    ) -> Result<OperationStatusResponse, ClientError> {
        let session = session.into();
        let prompt = prompt.into();
        match branch {
            Some(branch) => self.continue_agent(session, branch.into(), prompt),
            None => self.start_agent(session, prompt),
        }
    }

    /// Starts an agent operation for a session root.
    ///
    /// # Parameters
    /// - `session`: Stable target session identity.
    /// - `prompt`: User-authored agent instruction.
    ///
    /// # Returns
    /// The accepted shared operation status.
    ///
    /// # Errors
    /// Returns a structured server or transport error.
    pub fn start_agent(
        &self,
        session: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<OperationStatusResponse, ClientError> {
        self.start_agent_with_handles(session, prompt)
            .map(|response| response.operation)
    }

    /// Starts an agent operation and returns every accepted stable handle.
    ///
    /// # Parameters
    /// - `session`: Stable target session identity.
    /// - `prompt`: User-authored agent instruction.
    ///
    /// # Returns
    /// The typed accepted agent response with operation/session/branch handles.
    ///
    /// # Errors
    /// Returns a structured server or transport error.
    pub fn start_agent_with_handles(
        &self,
        session: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<scriberig_protocol::AgentAcceptedResponse, ClientError> {
        match self
            .send(Request::AgentStart(AgentStartRequest {
                session: SessionReference { id: session.into() },
                prompt: prompt.into(),
            }))?
            .response
        {
            Response::AgentAccepted(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Continues an agent operation on an existing branch.
    ///
    /// # Parameters
    /// - `session`: Stable target session identity.
    /// - `branch`: Stable target branch identity.
    /// - `prompt`: User-authored follow-up instruction.
    ///
    /// # Returns
    /// The accepted shared operation status.
    ///
    /// # Errors
    /// Returns a structured server or transport error.
    pub fn continue_agent(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<OperationStatusResponse, ClientError> {
        self.continue_agent_with_handles(session, branch, prompt)
            .map(|response| response.operation)
    }

    /// Continues an agent operation and returns every accepted stable handle.
    ///
    /// # Parameters
    /// - `session`: Stable target session identity.
    /// - `branch`: Stable target branch identity.
    /// - `prompt`: User-authored follow-up instruction.
    ///
    /// # Returns
    /// The typed accepted agent response with operation/session/branch handles.
    ///
    /// # Errors
    /// Returns a structured server or transport error.
    pub fn continue_agent_with_handles(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Result<scriberig_protocol::AgentAcceptedResponse, ClientError> {
        match self
            .send(Request::AgentContinue(AgentContinueRequest {
                session: SessionReference { id: session.into() },
                branch: BranchReference { id: branch.into() },
                prompt: prompt.into(),
            }))?
            .response
        {
            Response::AgentAccepted(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
