//! Dispatches protocol requests through the injected application boundary.

#[path = "dispatch/agent.rs"]
mod agent;
mod agent_console;
mod artifacts;
mod cad;
#[path = "dispatch/cad_bounds.rs"]
mod cad_bounds;
#[path = "dispatch/cad_find.rs"]
mod cad_find;
mod document;
mod geo;
mod geo_mapping;
mod helpers;
mod inspect;
mod plan;
mod projection_style;
mod qgis;
mod query;
mod request;
mod rich_request;
#[path = "dispatch/session.rs"]
mod session;
mod spreadsheet;

use helpers::{command_context, session_selection_context};
use scriberig_protocol::{
    CapabilitiesResponse, CapabilityInfo, ProtocolError, ProtocolErrorCode, ProtocolVersion,
    Request, RequestEnvelope, Response, ResponseEnvelope, RuntimeFeature,
};

use crate::server::ApplicationService;

/// Server-side protocol dispatcher for the initial vertical slice.
pub struct Dispatcher<S> {
    service: S,
    principal: Option<scriberig_app::Principal>,
    capabilities: scriberig_app::CapabilityContext,
}

impl<S> Dispatcher<S> {
    /// Creates a dispatcher around an application service.
    ///
    /// - `service`: Application implementation receiving semantic requests.
    ///
    /// A protocol dispatcher.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(service: S) -> Self {
        Self {
            service,
            principal: None,
            capabilities: scriberig_app::CapabilityContext::default(),
        }
    }

    /// Creates a dispatcher bound to a trusted caller identity.
    ///
    /// - `service`: Application implementation receiving semantic requests.
    /// - `principal`: Identity established by the trusted transport boundary.
    ///
    /// A protocol dispatcher carrying that caller identity.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn for_principal(service: S, principal: scriberig_app::Principal) -> Self {
        Self {
            service,
            principal: Some(principal),
            capabilities: scriberig_app::CapabilityContext::default(),
        }
    }

    /// Adds capability grants resolved by the trusted host for this dispatcher.
    ///
    /// Grants come from the host's authorization policy and are never accepted
    /// from protocol request data.
    ///
    /// - `capabilities`: Trusted scoped capability grants for dispatched requests.
    ///
    /// Dispatcher configured with host-authorized capability context.
    ///
    /// # Returns
    /// The configured dispatcher.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: scriberig_app::CapabilityContext) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Builds a request context from trusted dispatcher identity and grants.
    ///
    /// - `snapshot`: Optional exact snapshot selected by the protocol operation.
    /// - `correlation_id`: Request tracing identity.
    ///
    /// Application context containing only host-injected identity and grants.
    ///
    /// # Returns
    /// The application request context.
    fn application_context(
        &self,
        snapshot: Option<scriberig_core::SnapshotId>,
        correlation_id: &str,
    ) -> scriberig_app::ApplicationContext {
        scriberig_app::ApplicationContext {
            snapshot,
            principal: self.principal.clone(),
            capabilities: self.capabilities.clone(),
            correlation_id: Some(correlation_id.to_owned()),
            ..scriberig_app::ApplicationContext::default()
        }
    }
}

impl<S: ApplicationService> Dispatcher<S> {
    /// Dispatches one correlated protocol envelope.
    ///
    /// - `request`: Correlated protocol request.
    ///
    /// Correlated typed response or structured error.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn dispatch(&self, request: RequestEnvelope) -> ResponseEnvelope {
        self.dispatch_with_plan_lifecycle(request, None)
    }

    /// Dispatches a request and optionally runs plan lifecycle transitions.
    ///
    /// - `request`: Correlated protocol envelope.
    /// - `plan_lifecycle_handler`: Trusted plan worker runtime, when configured.
    ///
    /// Correlated typed response or structured error.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn dispatch_with_plan_lifecycle(
        &self,
        request: RequestEnvelope,
        plan_lifecycle_handler: Option<&crate::http::PlanLifecycleHandler<S>>,
    ) -> ResponseEnvelope {
        self.dispatch_with_runtime_handlers(request, None, plan_lifecycle_handler)
    }

    /// Dispatches a request through the configured host agent and plan runtimes.
    ///
    /// - `request`: Correlated protocol envelope.
    /// - `agent_execution_handler`: Trusted product runtime for agent requests.
    /// - `plan_lifecycle_handler`: Trusted plan worker runtime, when configured.
    ///
    /// Correlated typed response or structured error.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn dispatch_with_runtime_handlers(
        &self,
        request: RequestEnvelope,
        agent_execution_handler: Option<&crate::http::AgentExecutionHandler<S>>,
        plan_lifecycle_handler: Option<&crate::http::PlanLifecycleHandler<S>>,
    ) -> ResponseEnvelope {
        let correlation_id = request.correlation_id.clone();
        let response = if request.version.major == ProtocolVersion::default().major {
            self.dispatch_request(
                request.request,
                &correlation_id,
                agent_execution_handler,
                plan_lifecycle_handler,
            )
        } else {
            Response::Error(ProtocolError {
                code: ProtocolErrorCode::UnsupportedVersion,
                message: "unsupported protocol major version".to_owned(),
                details: Some(serde_json::json!({"major": request.version.major})),
                correlation_id: Some(correlation_id.clone()),
            })
        };
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id,
            response,
        }
    }

    /// Builds the capability response from the injected application service.
    ///
    /// The public runtime capability response.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Response {
        let capabilities = self
            .service
            .capabilities()
            .into_iter()
            .map(|capability| CapabilityInfo {
                id: capability.id.to_string(),
                description: capability.description,
                available: capability.available,
                domain: capability.scope,
            })
            .collect();
        Response::Capabilities(CapabilitiesResponse {
            protocol: ProtocolVersion::default(),
            capabilities,
            features: vec![RuntimeFeature {
                id: "inspect".to_owned(),
                enabled: true,
            }],
            cad_providers: self.service.cad_provider_capabilities(),
        })
    }
}
