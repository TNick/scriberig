//! Defines the transport-neutral in-process protocol envelope boundary.

use crate::{
    AgentContinueRequest, AgentStartRequest, CancelOperationRequest, CommandRequest, Event,
    FindRequest, ListRequest, QueryRequest, SessionReadRequest, SubscribeRequest,
};
use crate::{
    CapabilitiesResponse, InspectEntityRequest, InspectEntityResponse, OperationStatusRequest,
    OperationStatusResponse, ProtocolError, ProtocolVersion, RuntimeCapabilitiesRequest,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Initial requests supported by the remote vertical slice.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum Request {
    /// Inspect one semantic entity.
    Inspect(InspectEntityRequest),
    /// Discover runtime capabilities.
    Capabilities(RuntimeCapabilitiesRequest),
    /// Read one operation status.
    OperationStatus(OperationStatusRequest),
    /// Cancel one tracked operation.
    CancelOperation(CancelOperationRequest),
    /// Read events currently available to a subscription.
    Subscribe(SubscribeRequest),
    /// Evaluate one typed structural query.
    Query(QueryRequest),
    /// Read one bounded semantic collection page.
    List(ListRequest),
    /// Discover semantic entities within an exact scope.
    Find(FindRequest),
    /// Execute one typed application command.
    Command(CommandRequest),
    /// Start one provider-neutral agent operation.
    AgentStart(AgentStartRequest),
    /// Continue one provider-neutral agent operation.
    AgentContinue(AgentContinueRequest),
    /// Read one compact durable session tree.
    SessionRead(SessionReadRequest),
}

/// Initial typed responses supported by the remote vertical slice.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum Response {
    /// Inspection result.
    Inspect(InspectEntityResponse),
    /// Runtime capability result.
    Capabilities(CapabilitiesResponse),
    /// Operation status result.
    OperationStatus(OperationStatusResponse),
    /// Resulting state after cancellation.
    OperationCancelled(OperationStatusResponse),
    /// Events currently available to the subscriber.
    Events(Vec<Event>),
    /// Query result page.
    Query(crate::response::QueryResponse),
    /// Collection result page.
    List(crate::response::ListResponse),
    /// Semantic discovery result.
    Find(crate::response::FindResponse),
    /// Accepted command operation.
    CommandAccepted(OperationStatusResponse),
    /// Accepted agent operation.
    AgentAccepted(crate::AgentAcceptedResponse),
    /// Compact durable session tree.
    Session(crate::SessionReadResponse),
    /// Structured protocol failure.
    Error(ProtocolError),
}

/// Correlated protocol request envelope.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct RequestEnvelope {
    /// Protocol version requested by the client.
    pub version: ProtocolVersion,
    /// Correlation identity retained by the server.
    pub correlation_id: String,
    /// Typed semantic request.
    pub request: Request,
}

/// Correlated protocol response envelope.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ResponseEnvelope {
    /// Protocol version used by the server.
    pub version: ProtocolVersion,
    /// Correlation identity copied from the request.
    pub correlation_id: String,
    /// Typed response or structured error.
    pub response: Response,
}

/// Transport boundary independent of TCP/HTTP/WebSocket details.
pub trait ProtocolTransport {
    /// Sends one correlated protocol request.
    ///
    /// # Parameters
    /// - `request`: Request envelope to send.
    ///
    /// # Returns
    /// Correlated typed response envelope.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope;

    /// Returns and clears a transport failure from the most recent send.
    ///
    /// In-process transports never report one. Network transports may use this
    /// optional side channel because the protocol response itself is semantic.
    ///
    /// # Returns
    /// The most recent transport failure, if any.
    fn take_error(&self) -> Option<String> {
        None
    }
}
