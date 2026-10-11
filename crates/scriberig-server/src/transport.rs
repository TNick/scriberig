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
    /// # Parameters
    /// - `service`: Application implementation receiving semantic requests.
    ///
    /// # Returns
    /// An in-process protocol transport.
    #[must_use]
    pub fn new(service: S) -> Self {
        Self {
            dispatcher: Dispatcher::new(service),
        }
    }
}

impl<S: ApplicationService> ProtocolTransport for InProcessTransport<S> {
    /// Sends one request directly to the server dispatcher.
    ///
    /// # Parameters
    /// - `request`: Correlated protocol request.
    ///
    /// # Returns
    /// Correlated protocol response.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        self.dispatcher.dispatch(request)
    }
}
