//! Provides the public in-process project protocol transport.

use std::sync::Arc;

use scriberig_protocol::{ProtocolError, ProtocolErrorCode, ProtocolTransport};
use serde_json::Value;

use super::{HttpState, http_protocol};
use crate::server::ApplicationService;

/// Runs project protocol requests through the server adapter in process.
///
/// This transport is intended for public cross-surface conformance tests. It
/// retains the protocol/client boundary while avoiding platform-dependent
/// loopback sockets in tests.
pub struct InProcessProjectTransport<S> {
    /// Server state receiving protocol requests.
    state: Arc<HttpState<S>>,
}

impl<S> InProcessProjectTransport<S> {
    /// Creates a project transport over one server state.
    ///
    /// # Parameters
    /// - `state`: Public server state receiving project requests.
    ///
    /// # Returns
    /// An in-process protocol transport.
    #[must_use]
    pub fn new(state: Arc<HttpState<S>>) -> Self {
        Self { state }
    }
}

impl<S: ApplicationService> ProtocolTransport for InProcessProjectTransport<S> {
    /// Dispatches one typed project request through the HTTP protocol adapter.
    ///
    /// # Parameters
    /// - `request`: Protocol envelope to dispatch.
    ///
    /// # Returns
    /// The decoded public protocol response envelope.
    fn send(
        &self,
        request: scriberig_protocol::RequestEnvelope,
    ) -> scriberig_protocol::ResponseEnvelope {
        let correlation_id = request.correlation_id.clone();
        let scriberig_protocol::Request::Project(project) = request.request else {
            return protocol_error(
                correlation_id,
                "in-process project transport received a non-project request".to_owned(),
            );
        };
        let body = match serde_json::to_string(&project) {
            Ok(body) => body,
            Err(error) => return protocol_error(correlation_id, error.to_string()),
        };
        let result = http_protocol::dispatch(&self.state, "project", &body, correlation_id.clone());
        match result {
            Ok(value) => serde_json::from_value(value)
                .unwrap_or_else(|error| protocol_error(correlation_id, error.to_string())),
            Err((_, value)) => protocol_error(
                correlation_id,
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("project operation failed")
                    .to_owned(),
            ),
        }
    }

    /// Reports no transport error because dispatch is in process.
    ///
    /// # Returns
    /// Always `None`; dispatch errors are protocol responses.
    fn take_error(&self) -> Option<String> {
        None
    }
}

/// Builds a protocol error response for in-process transport failures.
///
/// # Parameters
/// - `correlation_id`: Request correlation identity.
/// - `message`: Safe failure description.
///
/// # Returns
/// A protocol-shaped internal error response.
fn protocol_error(correlation_id: String, message: String) -> scriberig_protocol::ResponseEnvelope {
    scriberig_protocol::ResponseEnvelope {
        version: scriberig_protocol::ProtocolVersion::default(),
        correlation_id,
        response: scriberig_protocol::Response::Error(ProtocolError {
            code: ProtocolErrorCode::Internal,
            message,
            details: None,
            correlation_id: None,
        }),
    }
}
