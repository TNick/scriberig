//! Provides semantic client methods over a transport-neutral protocol boundary.

use std::sync::atomic::{AtomicU64, Ordering};

use scriberig_protocol::{
    CapabilitiesResponse, EntityReference, InspectEntityRequest, InspectEntityResponse,
    OperationReference, OperationStatusRequest, OperationStatusResponse, ProtocolTransport,
    ProtocolVersion, Request, RequestEnvelope, Response, RuntimeCapabilitiesRequest,
    SnapshotSelector,
};

use crate::error::ClientError;

/// Reusable semantic client that hides protocol envelope details.
pub struct Client<T> {
    transport: T,
    next_correlation: AtomicU64,
}

impl<T: ProtocolTransport> Client<T> {
    /// Creates a client over a protocol transport.
    ///
    /// # Parameters
    /// - `transport`: Transport implementation used for requests.
    ///
    /// # Returns
    /// A semantic protocol client.
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_correlation: AtomicU64::new(1),
        }
    }

    /// Inspects one entity at an explicit snapshot selector.
    ///
    /// # Parameters
    /// - `entity`: Transport-safe entity identifier.
    /// - `snapshot`: Snapshot resolution rule.
    /// - `deep`: Whether detailed summary fields are requested.
    ///
    /// # Returns
    /// Snapshot-bound inspection response.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
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
        match self.send(request).response {
            Response::Inspect(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Retrieves runtime capability metadata.
    ///
    /// # Returns
    /// Current capability response.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn capabilities(&self) -> Result<CapabilitiesResponse, ClientError> {
        match self
            .send(Request::Capabilities(RuntimeCapabilitiesRequest::default()))
            .response
        {
            Response::Capabilities(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Retrieves one long-running operation status.
    ///
    /// # Parameters
    /// - `operation`: Transport-safe operation identifier.
    ///
    /// # Returns
    /// Current operation status response.
    ///
    /// # Errors
    /// Returns a structured server error or response mismatch.
    pub fn operation_status(
        &self,
        operation: impl Into<String>,
    ) -> Result<OperationStatusResponse, ClientError> {
        match self
            .send(Request::OperationStatus(OperationStatusRequest {
                operation: OperationReference {
                    id: operation.into(),
                },
            }))
            .response
        {
            Response::OperationStatus(response) => Ok(response),
            Response::Error(error) => Err(ClientError::Protocol(error)),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Sends one request with protocol and correlation metadata.
    ///
    /// # Parameters
    /// - `request`: Typed protocol request.
    ///
    /// # Returns
    /// Correlated transport response.
    fn send(&self, request: Request) -> scriberig_protocol::ResponseEnvelope {
        let correlation = self.next_correlation.fetch_add(1, Ordering::Relaxed);
        self.transport.send(RequestEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: format!("client-{correlation}"),
            request,
        })
    }
}
