//! Provides semantic client methods for durable session detail.

use super::Client;
use crate::error::ClientError;
use scriberig_protocol::{
    AgentPresentationPreferencesReadRequest, AgentPresentationPreferencesResponse,
    AgentPresentationPreferencesSetRequest, BranchReference, ModelRequestDetailRequest,
    ModelRequestDetailResponse, ProtocolTransport, PublicAgentPresentationPreferences, Request,
    Response, SessionReadRequest, SessionReadResponse, SessionReference, ToolResultDetailRequest,
    ToolResultDetailResponse,
};

impl<T: ProtocolTransport> Client<T> {
    /// Reads shared presentation preferences for the authenticated principal.
    ///
    /// # Parameters
    /// - `self`: Client using the protocol transport.
    ///
    /// # Returns
    /// Revisioned shared presentation defaults.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn agent_presentation_preferences(
        &self,
    ) -> Result<AgentPresentationPreferencesResponse, ClientError> {
        match self
            .send(Request::AgentPresentationPreferencesRead(
                AgentPresentationPreferencesReadRequest {},
            ))?
            .response
        {
            Response::AgentPresentationPreferences(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Replaces shared preferences at the caller's observed revision.
    ///
    /// # Parameters
    /// - `self`: Client using the protocol transport.
    /// - `expected_revision`: Revision from the last preference read.
    /// - `preferences`: Complete set of shared defaults.
    ///
    /// # Returns
    /// Updated revisioned shared presentation defaults.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn set_agent_presentation_preferences(
        &self,
        expected_revision: u64,
        preferences: PublicAgentPresentationPreferences,
    ) -> Result<AgentPresentationPreferencesResponse, ClientError> {
        match self
            .send(Request::AgentPresentationPreferencesSet(
                AgentPresentationPreferencesSetRequest {
                    expected_revision,
                    preferences,
                },
            ))?
            .response
        {
            Response::AgentPresentationPreferences(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads one branch and its nested descendants on demand.
    ///
    /// # Parameters
    /// - `self`: Client using the protocol transport.
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity whose activity is requested.
    ///
    /// # Returns
    /// The selected branch subtree without unrelated branch histories.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn read_session_branch(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
    ) -> Result<SessionReadResponse, ClientError> {
        match self
            .send(Request::SessionRead(SessionReadRequest {
                session: SessionReference { id: session.into() },
                branch: Some(BranchReference { id: branch.into() }),
            }))?
            .response
        {
            Response::Session(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Loads the captured safe model request for one invocation.
    ///
    /// # Parameters
    /// - `self`: Client using the protocol transport.
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity.
    /// - `invocation`: Model invocation activity identity.
    ///
    /// # Returns
    /// Bounded provider-neutral request detail retained by the session.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn model_request_detail(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
        invocation: impl Into<String>,
    ) -> Result<ModelRequestDetailResponse, ClientError> {
        match self
            .send(Request::ModelRequestDetail(ModelRequestDetailRequest {
                session: SessionReference { id: session.into() },
                branch: BranchReference { id: branch.into() },
                invocation: invocation.into(),
            }))?
            .response
        {
            Response::ModelRequestDetail(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Loads bounded safe detail for one durable tool result.
    ///
    /// # Parameters
    /// - `self`: Client using the protocol transport.
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity.
    /// - `result`: Tool result activity identity.
    ///
    /// # Returns
    /// Whitelisted safe result fields and a truncation indicator.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn tool_result_detail(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
        result: impl Into<String>,
    ) -> Result<ToolResultDetailResponse, ClientError> {
        match self
            .send(Request::ToolResultDetail(ToolResultDetailRequest {
                session: SessionReference { id: session.into() },
                branch: BranchReference { id: branch.into() },
                result: result.into(),
            }))?
            .response
        {
            Response::ToolResultDetail(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
