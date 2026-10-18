//! Provides agent-operation methods for the protocol client.

use super::{
    AgentContinueRequest, AgentStartRequest, BranchReference, Client, ClientError,
    OperationStatusResponse, ProtocolTransport, Request, Response, SessionReference,
};
use scriberig_protocol::{
    ModelLibraryRequest, ModelLibraryResponse, SessionExecutionSelection,
    SessionModelSelectionRequest, SessionThinkingSelectionRequest,
};

impl<T: ProtocolTransport> Client<T> {
    /// Retrieves safe model profiles and model-specific thinking capabilities.
    ///
    /// # Returns
    /// The configured provider-neutral model library.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn model_library(&self) -> Result<ModelLibraryResponse, ClientError> {
        match self
            .send(Request::ModelLibrary(ModelLibraryRequest::default()))?
            .response
        {
            Response::ModelLibrary(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Selects a model profile for future invocations in one session.
    ///
    /// # Parameters
    /// - `session`: Stable session identity.
    /// - `expected_revision`: Selection revision observed by the caller.
    /// - `model_profile`: Curated profile identity to select.
    ///
    /// # Returns
    /// Updated session execution selection.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn set_session_model(
        &self,
        session: impl Into<String>,
        expected_revision: u64,
        model_profile: impl Into<String>,
    ) -> Result<SessionExecutionSelection, ClientError> {
        match self
            .send(Request::SessionModelSelection(
                SessionModelSelectionRequest {
                    session: SessionReference { id: session.into() },
                    expected_revision,
                    model_profile: model_profile.into(),
                },
            ))?
            .response
        {
            Response::SessionExecution(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Selects a thinking capability for future invocations in one session.
    ///
    /// # Parameters
    /// - `session`: Stable session identity.
    /// - `expected_revision`: Selection revision observed by the caller.
    /// - `thinking_level`: Advertised thinking-level identity.
    ///
    /// # Returns
    /// Updated session execution selection.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn set_session_thinking_level(
        &self,
        session: impl Into<String>,
        expected_revision: u64,
        thinking_level: impl Into<String>,
    ) -> Result<SessionExecutionSelection, ClientError> {
        match self
            .send(Request::SessionThinkingSelection(
                SessionThinkingSelectionRequest {
                    session: SessionReference { id: session.into() },
                    expected_revision,
                    thinking_level: thinking_level.into(),
                },
            ))?
            .response
        {
            Response::SessionExecution(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

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
                selection: None,
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
                selection: None,
            }))?
            .response
        {
            Response::AgentAccepted(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}

#[cfg(test)]
#[path = "client_agent_tests.rs"]
mod tests;
