//! Provides an in-process implementation of the protocol transport boundary.

use scriberig_protocol::{ProtocolTransport, RequestEnvelope, ResponseEnvelope};

use crate::dispatch::Dispatcher;
use crate::server::ApplicationService;

/// In-process transport used by integration tests and embedded deployments.
pub struct InProcessTransport<S> {
    dispatcher: Dispatcher<S>,
}

impl<S> InProcessTransport<S> {
    /// Creates an in-process transport around an application service.
    ///
    /// - `service`: Application implementation receiving semantic requests.
    ///
    /// An in-process protocol transport.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(service: S) -> Self {
        Self {
            dispatcher: Dispatcher::new(service),
        }
    }

    /// Creates an in-process transport bound to a trusted caller identity.
    ///
    /// - `service`: Application implementation receiving semantic requests.
    /// - `principal`: Caller identity established by the embedding runtime.
    ///
    /// An in-process protocol transport with principal-scoped operations.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn for_principal(service: S, principal: scriberig_app::Principal) -> Self {
        Self {
            dispatcher: Dispatcher::for_principal(service, principal),
        }
    }
}

impl<S: ApplicationService> ProtocolTransport for InProcessTransport<S> {
    /// Sends one request directly to the server dispatcher.
    ///
    /// - `request`: Correlated protocol request.
    ///
    /// Correlated protocol response.
    /// # Returns
    /// The operation result.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        self.dispatcher.dispatch(request)
    }
}
