//! Provides an in-process implementation of the protocol transport boundary.

use scriberig_protocol::{ProtocolTransport, RequestEnvelope, ResponseEnvelope};

use crate::dispatch::Dispatcher;
use crate::server::ApplicationService;

/// In-process transport used by integration tests and embedded deployments.
pub struct InProcessTransport<S> {
    dispatcher: Dispatcher<S>,
    agent_execution_handler: Option<std::sync::Arc<crate::AgentExecutionHandler<S>>>,
}

impl<S> InProcessTransport<S> {
    /// Adds grants resolved by the trusted embedding host.
    ///
    /// - `capabilities`: Scoped grants established outside protocol input.
    ///
    /// # Returns
    /// Transport whose dispatcher evaluates requests with these grants.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: scriberig_app::CapabilityContext) -> Self {
        self.dispatcher = self.dispatcher.with_capabilities(capabilities);
        self
    }

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
            agent_execution_handler: None,
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
            agent_execution_handler: None,
        }
    }

    /// Attaches the trusted runtime used to execute accepted agent requests.
    ///
    /// This mirrors the HTTP composition path while keeping requests in process,
    /// which lets embedded protocol clients exercise the same host runtime.
    ///
    /// - `self`: In-process transport receiving the runtime.
    /// - `handler`: Host-owned agent execution handler.
    ///
    /// The transport configured to run accepted requests through `handler`.
    ///
    /// # Returns
    /// The transport configured with the host-owned runtime.
    #[must_use]
    pub fn with_agent_execution_handler(
        mut self,
        handler: std::sync::Arc<crate::AgentExecutionHandler<S>>,
    ) -> Self {
        self.agent_execution_handler = Some(handler);
        self
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
        self.dispatcher.dispatch_with_runtime_handlers(
            request,
            self.agent_execution_handler.as_deref(),
            None,
        )
    }
}
