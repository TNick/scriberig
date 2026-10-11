//! Defines the transport-neutral in-process protocol envelope boundary.

use crate::{
    CapabilitiesResponse, InspectEntityRequest, InspectEntityResponse, OperationStatusRequest,
    OperationStatusResponse, ProtocolError, ProtocolVersion, RuntimeCapabilitiesRequest,
};
use serde::{Deserialize, Serialize};

/// Initial requests supported by the remote vertical slice.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Request {
    /// Inspect one semantic entity.
    Inspect(InspectEntityRequest),
    /// Discover runtime capabilities.
    Capabilities(RuntimeCapabilitiesRequest),
    /// Read one operation status.
    OperationStatus(OperationStatusRequest),
}

/// Initial typed responses supported by the remote vertical slice.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Response {
    /// Inspection result.
    Inspect(InspectEntityResponse),
    /// Runtime capability result.
    Capabilities(CapabilitiesResponse),
    /// Operation status result.
    OperationStatus(OperationStatusResponse),
    /// Structured protocol failure.
    Error(ProtocolError),
}

/// Correlated protocol request envelope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequestEnvelope {
    /// Protocol version requested by the client.
    pub version: ProtocolVersion,
    /// Correlation identity retained by the server.
    pub correlation_id: String,
    /// Typed semantic request.
    pub request: Request,
}

/// Correlated protocol response envelope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
}
