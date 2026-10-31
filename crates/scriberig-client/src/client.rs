//! Provides semantic client methods over a transport-neutral protocol boundary.

use std::sync::atomic::{AtomicU64, Ordering};

use scriberig_protocol::{
    AgentContinueRequest, AgentStartRequest, BranchReference, CancelOperationRequest,
    CapabilitiesResponse, CommandRequest, DomainOperationCommand, EntityReference, Event,
    InspectEntityRequest, InspectEntityResponse, OperationReference, OperationStatusRequest,
    OperationStatusResponse, ProtocolTransport, ProtocolVersion, QueryRequest, Request,
    RequestEnvelope, Response, RuntimeCapabilitiesRequest, SessionReference, SnapshotSelector,
    SubscribeRequest,
};

#[path = "client_agent.rs"]
mod client_agent;
#[path = "client_agent_recovery.rs"]
mod client_agent_recovery;
#[path = "client_cad.rs"]
mod client_cad;
#[path = "client_document.rs"]
mod client_document;
#[path = "client_geo.rs"]
mod client_geo;
#[path = "client_project.rs"]
mod client_project;
#[path = "client_projection_style.rs"]
mod client_projection_style;
#[path = "client_session.rs"]
mod client_session;
#[path = "client_session_read.rs"]
mod client_session_read;
#[path = "client_spreadsheet.rs"]
mod client_spreadsheet;
use scriberig_query::{QueryResult, QuerySpec};

use crate::error::ClientError;

/// Reusable semantic client that hides protocol envelope details.
pub struct Client<T> {
    transport: T,
    next_correlation: AtomicU64,
}

impl<T: ProtocolTransport> Client<T> {
    /// Creates a client over a protocol transport.
    ///
    /// - `transport`: Transport implementation used for requests.
    ///
    /// A semantic protocol client.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_correlation: AtomicU64::new(1),
        }
    }

    /// Inspects one entity at an explicit snapshot selector.
    ///
    /// - `entity`: Transport-safe entity identifier.
    /// - `snapshot`: Snapshot resolution rule.
    /// - `deep`: Whether detailed summary fields are requested.
    ///
    /// Snapshot-bound inspection response.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn inspect(
        &self,
        entity: impl Into<String>,
        snapshot: SnapshotSelector,
        deep: bool,
    ) -> Result<InspectEntityResponse, ClientError> {
        let request = Request::Inspect(InspectEntityRequest {
            entity: EntityReference { id: entity.into() },
            snapshot,
            detail: if deep {
                scriberig_protocol::request::DetailLevel::Deep
            } else {
                scriberig_protocol::request::DetailLevel::Compact
            },
        });
        match self.send(request)?.response {
            Response::Inspect(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Retrieves runtime capability metadata.
    ///
    /// Current capability response.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn capabilities(&self) -> Result<CapabilitiesResponse, ClientError> {
        match self
            .send(Request::Capabilities(RuntimeCapabilitiesRequest::default()))?
            .response
        {
            Response::Capabilities(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Retrieves one long-running operation status.
    ///
    /// - `operation`: Transport-safe operation identifier.
    ///
    /// Current operation status response.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn operation_status(
        &self,
        operation: impl Into<String>,
    ) -> Result<OperationStatusResponse, ClientError> {
        match self
            .send(Request::OperationStatus(OperationStatusRequest {
                operation: OperationReference {
                    id: operation.into(),
                },
            }))?
            .response
        {
            Response::OperationStatus(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Requests cancellation of one tracked operation.
    ///
    /// - `operation`: Transport-safe operation identifier.
    ///
    /// The operation's cancelled lifecycle state.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn cancel_operation(
        &self,
        operation: impl Into<String>,
    ) -> Result<OperationStatusResponse, ClientError> {
        match self
            .send(Request::CancelOperation(CancelOperationRequest {
                operation: OperationReference {
                    id: operation.into(),
                },
            }))?
            .response
        {
            Response::OperationCancelled(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads currently available events matching a subscription.
    ///
    /// - `event_kinds`: Optional event names; empty selects all events.
    ///
    /// Stable event envelopes available at the time of the request.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn subscribe(&self, event_kinds: Vec<String>) -> Result<Vec<Event>, ClientError> {
        self.subscribe_after(event_kinds, None)
    }

    /// Reads events published after an optional protocol cursor.
    ///
    /// - `event_kinds`: Optional event names; empty selects all events.
    /// - `after`: Last cursor already processed by the caller.
    ///
    /// Stable event envelopes newer than the requested cursor.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn subscribe_after(
        &self,
        event_kinds: Vec<String>,
        after: Option<u64>,
    ) -> Result<Vec<Event>, ClientError> {
        match self
            .send(Request::Subscribe(SubscribeRequest { event_kinds, after }))?
            .response
        {
            Response::Events(events) => Ok(events),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Evaluates one structured query through the remote application boundary.
    ///
    /// - `specification`: Snapshot-bound structural query.
    ///
    /// A typed query page retaining its exact scope.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn query(&self, specification: QuerySpec) -> Result<QueryResult, ClientError> {
        match self
            .send(Request::Query(QueryRequest {
                specification: serde_json::to_value(specification)
                    .map_err(|_| ClientError::UnexpectedResponse)?,
            }))?
            .response
        {
            Response::Query(response) => {
                serde_json::from_value(response.result).map_err(|_| ClientError::UnexpectedResponse)
            }
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads one bounded collection page through the remote application
    /// boundary.
    ///
    /// - `specification`: Snapshot-bound universal list request.
    ///
    /// The typed collection page.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn list(
        &self,
        specification: scriberig_query::ListRequest,
    ) -> Result<scriberig_query::ListResult, ClientError> {
        match self
            .send(Request::List(scriberig_protocol::ListRequest {
                specification: serde_json::to_value(specification)
                    .map_err(|_| ClientError::UnexpectedResponse)?,
            }))?
            .response
        {
            Response::List(response) => {
                serde_json::from_value(response.result).map_err(|_| ClientError::UnexpectedResponse)
            }
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Executes bounded semantic discovery through the remote application
    /// boundary.
    ///
    /// - `specification`: Snapshot-scoped universal find request.
    ///
    /// The typed discovery result.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn find(
        &self,
        specification: scriberig_query::FindRequest,
    ) -> Result<scriberig_query::FindResult, ClientError> {
        match self
            .send(Request::Find(scriberig_protocol::FindRequest {
                specification: serde_json::to_value(specification)
                    .map_err(|_| ClientError::UnexpectedResponse)?,
            }))?
            .response
        {
            Response::Find(response) => {
                serde_json::from_value(response.result).map_err(|_| ClientError::UnexpectedResponse)
            }
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Sends a presentation-neutral application command payload.
    ///
    /// - `kind`: Stable command family identifier.
    /// - `payload`: Typed command payload encoded as JSON.
    ///
    /// The tracked operation accepted by the application boundary.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn command(
        &self,
        kind: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<OperationStatusResponse, ClientError> {
        match self
            .send(Request::Command(CommandRequest {
                kind: kind.into(),
                payload,
            }))?
            .response
        {
            Response::CommandAccepted(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Executes a registered semantic operation through the shared command API.
    ///
    /// - `request`: Exact semantic target, operation, and scalar arguments.
    ///
    /// The tracked operation accepted by the application boundary.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn domain_operation(
        &self,
        request: DomainOperationCommand,
    ) -> Result<OperationStatusResponse, ClientError> {
        if let Some(anchor) = &request.projection_anchor {
            anchor
                .validate()
                .map_err(|error| ClientError::Encoding(error.to_owned()))?;
        }
        self.command(
            "domain_operation",
            serde_json::to_value(request)
                .map_err(|error| ClientError::Encoding(error.to_string()))?,
        )
    }

    /// Sends one request with protocol and correlation metadata.
    ///
    /// - `request`: Typed protocol request.
    ///
    /// Correlated transport response.
    ///
    /// Returns a transport or protocol error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(crate) fn send(
        &self,
        request: Request,
    ) -> Result<scriberig_protocol::ResponseEnvelope, ClientError> {
        let correlation = self.next_correlation.fetch_add(1, Ordering::Relaxed);
        let response = self.transport.send(RequestEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: format!("client-{correlation}"),
            request,
        });
        if let Some(error) = self.transport.take_error() {
            return Err(ClientError::Transport(error));
        }
        Ok(response)
    }
}
