//! Dispatches protocol requests through the injected application boundary.

mod agent_console;
mod helpers;
mod inspect;
mod query;
#[path = "dispatch/session.rs"]
mod session;

use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityId, CapabilityScope, OperationKind,
};
use scriberig_protocol::{
    CapabilitiesResponse, CapabilityInfo, OperationStatusResponse, ProtocolError,
    ProtocolErrorCode, ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
    RuntimeFeature,
};

use crate::server::ApplicationService;

/// Server-side protocol dispatcher for the initial vertical slice.
pub struct Dispatcher<S> {
    service: S,
    principal: Option<scriberig_app::Principal>,
}

impl<S> Dispatcher<S> {
    /// Creates a dispatcher around an application service.
    ///
    /// # Parameters
    /// - `service`: Application implementation receiving semantic requests.
    ///
    /// # Returns
    /// A protocol dispatcher.
    #[must_use]
    pub fn new(service: S) -> Self {
        Self {
            service,
            principal: None,
        }
    }

    /// Creates a dispatcher bound to a trusted caller identity.
    ///
    /// # Parameters
    /// - `service`: Application implementation receiving semantic requests.
    /// - `principal`: Identity established by the trusted transport boundary.
    ///
    /// # Returns
    /// A protocol dispatcher carrying that caller identity.
    #[must_use]
    pub fn for_principal(service: S, principal: scriberig_app::Principal) -> Self {
        Self {
            service,
            principal: Some(principal),
        }
    }
}

impl<S: ApplicationService> Dispatcher<S> {
    /// Dispatches one correlated protocol envelope.
    ///
    /// # Parameters
    /// - `request`: Correlated protocol request.
    ///
    /// # Returns
    /// Correlated typed response or structured error.
    #[must_use]
    pub fn dispatch(&self, request: RequestEnvelope) -> ResponseEnvelope {
        let correlation_id = request.correlation_id.clone();
        let response = if request.version.major == ProtocolVersion::default().major {
            self.dispatch_request(request.request, &correlation_id)
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

    /// Dispatches a version-validated typed request.
    ///
    /// # Parameters
    /// - `request`: Version-validated typed request.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// Typed protocol response.
    #[allow(clippy::too_many_lines)]
    fn dispatch_request(&self, request: Request, correlation_id: &str) -> Response {
        match request {
            Request::Inspect(request) => inspect::inspect(&self.service, request, correlation_id),
            Request::Capabilities(_) => self.capabilities(),
            Request::OperationStatus(request) => self
                .service
                .operation_status(&request.operation.id)
                .map_or_else(
                    |error| Response::Error(helpers::map_error(&error, correlation_id)),
                    |operation| Response::OperationStatus(OperationStatusResponse { operation }),
                ),
            Request::CancelOperation(request) => self
                .service
                .cancel_operation(&request.operation.id)
                .map_or_else(
                    |error| Response::Error(helpers::map_error(&error, correlation_id)),
                    |operation| Response::OperationCancelled(OperationStatusResponse { operation }),
                ),
            Request::Subscribe(request) => self
                .service
                .events_after(&request.event_kinds, request.after.unwrap_or_default())
                .map_or_else(
                    |error| Response::Error(helpers::map_error(&error, correlation_id)),
                    Response::Events,
                ),
            Request::Query(request) => {
                query::query(&self.service, request.specification, correlation_id)
            }
            Request::List(request) => {
                query::list(&self.service, request.specification, correlation_id)
            }
            Request::Find(request) => {
                query::find(&self.service, request.specification, correlation_id)
            }
            Request::Command(request) => self
                .service
                .command(
                    &command_context(&request.kind),
                    &request.kind,
                    &request.payload,
                )
                .map_or_else(
                    |error| Response::Error(helpers::map_error(&error, correlation_id)),
                    |operation| Response::CommandAccepted(OperationStatusResponse { operation }),
                ),
            Request::AgentStart(request) => self.agent_command(
                "agent_start",
                &serde_json::json!({
                    "session": request.session.id,
                    "prompt": request.prompt,
                    "selection": request.selection,
                }),
                None,
                correlation_id,
            ),
            Request::AgentContinue(request) => self.agent_command(
                "agent_continue",
                &serde_json::json!({
                    "session": request.session.id,
                    "branch": request.branch.id,
                    "prompt": request.prompt,
                    "selection": request.selection,
                }),
                Some((&request.session, &request.branch)),
                correlation_id,
            ),
            Request::SessionRead(request) => self.session_read(&request, correlation_id),
            Request::ModelRequestDetail(request) => self
                .service
                .model_request_detail(
                    &command_context("session_branch_tree"),
                    &request.session.id,
                    &request.branch.id,
                    &request.invocation,
                )
                .map_or_else(
                    |error| Response::Error(helpers::map_error(&error, correlation_id)),
                    Response::ModelRequestDetail,
                ),
            Request::ToolResultDetail(request) => self
                .service
                .tool_result_detail(
                    &command_context("session_branch_tree"),
                    &request.session.id,
                    &request.branch.id,
                    &request.result,
                )
                .map_or_else(
                    |error| Response::Error(helpers::map_error(&error, correlation_id)),
                    Response::ToolResultDetail,
                ),
            Request::ModelLibrary(_) => agent_console::model_library(
                &self.service,
                &command_context("agent_model_library"),
                correlation_id,
            ),
            Request::AgentPresentationPreferencesRead(_) => agent_console::read_preferences(
                &self.service,
                &agent_console::presentation_context(self.principal.as_ref()),
                correlation_id,
            ),
            Request::AgentPresentationPreferencesSet(request) => agent_console::set_preferences(
                &self.service,
                &agent_console::presentation_context(self.principal.as_ref()),
                &request,
                correlation_id,
            ),
            Request::SessionModelSelection(request) => {
                agent_console::set_model(&self.service, &request, correlation_id)
            }
            Request::SessionThinkingSelection(request) => {
                agent_console::set_thinking(&self.service, &request, correlation_id)
            }
            Request::Project(_) => Response::Error(ProtocolError {
                code: ProtocolErrorCode::UnsupportedOperation,
                message: "project operations require the HTTP project catalog".to_owned(),
                details: None,
                correlation_id: Some(correlation_id.to_owned()),
            }),
        }
    }

    /// Builds the least-capability context for principal-scoped display settings.
    ///
    /// # Returns
    /// Context restricted to the model-library capability and bound to the
    /// trusted transport principal, when one was configured.
    /// Builds the capability response from the injected application service.
    ///
    /// # Returns
    /// The public runtime capability response.
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
        })
    }

    /// Dispatches one typed agent request through the shared command boundary.
    ///
    /// # Parameters
    /// - `kind`: Stable agent operation identifier.
    /// - `payload`: Provider-neutral request payload.
    /// - `references`: Session and optional branch references for the response.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// Typed accepted agent response or protocol error.
    fn agent_command(
        &self,
        kind: &str,
        payload: &serde_json::Value,
        references: Option<(
            &scriberig_protocol::SessionReference,
            &scriberig_protocol::BranchReference,
        )>,
        correlation_id: &str,
    ) -> Response {
        if !self
            .service
            .capabilities()
            .iter()
            .any(|capability| capability.id.as_str() == kind && capability.available)
        {
            return Response::Error(helpers::map_error(
                &scriberig_app::ApplicationError::UnsupportedOperation {
                    operation: kind.to_owned(),
                },
                correlation_id,
            ));
        }
        let session = references.as_ref().map_or_else(
            || {
                payload
                    .get("session")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            },
            |(session, _)| session.id.clone(),
        );
        self.service
            .command(&command_context(kind), kind, payload)
            .map_or_else(
                |error| Response::Error(helpers::map_error(&error, correlation_id)),
                |operation| {
                    let branch = references.map(|(_, branch)| branch.clone()).or_else(|| {
                        self.service
                            .read_session(&command_context("session_branch_tree"), &session)
                            .ok()
                            .and_then(|value| value.branches.last().cloned())
                            .map(|branch| scriberig_protocol::BranchReference {
                                id: branch.id.clone(),
                            })
                    });
                    Response::AgentAccepted(scriberig_protocol::AgentAcceptedResponse {
                        operation: OperationStatusResponse { operation },
                        session: scriberig_protocol::SessionReference { id: session },
                        branch,
                    })
                },
            )
    }
}

/// Builds the trusted server context for an exposed command family.
///
/// # Parameters
/// - `kind`: Stable command or capability identifier.
///
/// # Returns
/// A context granting only the requested public command capability.
fn command_context(kind: &str) -> ApplicationContext {
    let mut capabilities = vec![CapabilityId::new(kind).expect("protocol command capability")];
    if matches!(kind, "agent_start" | "agent_continue") {
        capabilities.push(CapabilityId::new("session_branch_tree").expect("session capability"));
    }
    ApplicationContext {
        capabilities: CapabilityContext::new(capabilities),
        ..ApplicationContext::empty()
    }
}

/// Grants the trusted session-selection action for one exact session.
///
/// # Parameters
/// - `session`: Stable session identity from the protocol request.
///
/// # Returns
/// Application context authorizing session model configuration.
fn session_selection_context(session: &str) -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext {
            granted: vec![
                CapabilityId::new("session_branch_tree").expect("session read capability"),
            ],
            grants: vec![Capability {
                scope: CapabilityScope::Session(session.to_owned()),
                actions: [CapabilityAction::ModifyBranch].into_iter().collect(),
                constraints: CapabilityConstraints {
                    operation: Some(OperationKind::Agent),
                    ..CapabilityConstraints::unrestricted()
                },
            }],
        },
        ..ApplicationContext::empty()
    }
}
