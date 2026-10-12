//! Dispatches protocol requests through the injected application boundary.

mod helpers;

use std::str::FromStr;

use scriberig_app::{
    ApplicationContext, CapabilityContext, CapabilityId, DetailLevel, InspectRequest,
    SnapshotResolution,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{
    CapabilitiesResponse, CapabilityInfo, OperationStatusResponse, ProtocolError,
    ProtocolErrorCode, ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
    RuntimeFeature,
};
use scriberig_query::QuerySpec;

use crate::server::ApplicationService;

/// Server-side protocol dispatcher for the initial vertical slice.
pub struct Dispatcher<S> {
    service: S,
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
        Self { service }
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
            Request::Inspect(request) => self.inspect(request, correlation_id),
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
                let Ok(specification) = serde_json::from_value::<QuerySpec>(request.specification)
                else {
                    return Response::Error(ProtocolError {
                        code: ProtocolErrorCode::SchemaViolation,
                        message: "invalid query specification".to_owned(),
                        details: None,
                        correlation_id: Some(correlation_id.to_owned()),
                    });
                };
                self.service
                    .query(&ApplicationContext::empty(), &specification)
                    .map_or_else(
                        |error| Response::Error(helpers::map_error(&error, correlation_id)),
                        |result| {
                            Response::Query(scriberig_protocol::QueryResponse {
                                result: serde_json::to_value(result).unwrap_or_default(),
                            })
                        },
                    )
            }
            Request::List(request) => {
                let Ok(specification) =
                    serde_json::from_value::<scriberig_query::ListRequest>(request.specification)
                else {
                    return Response::Error(ProtocolError {
                        code: ProtocolErrorCode::SchemaViolation,
                        message: "invalid list specification".to_owned(),
                        details: None,
                        correlation_id: Some(correlation_id.to_owned()),
                    });
                };
                self.service
                    .list(&ApplicationContext::empty(), &specification)
                    .map_or_else(
                        |error| Response::Error(helpers::map_error(&error, correlation_id)),
                        |result| {
                            Response::List(scriberig_protocol::ListResponse {
                                result: serde_json::to_value(result).unwrap_or_default(),
                            })
                        },
                    )
            }
            Request::Find(request) => {
                let Ok(specification) =
                    serde_json::from_value::<scriberig_query::FindRequest>(request.specification)
                else {
                    return Response::Error(ProtocolError {
                        code: ProtocolErrorCode::SchemaViolation,
                        message: "invalid find specification".to_owned(),
                        details: None,
                        correlation_id: Some(correlation_id.to_owned()),
                    });
                };
                self.service
                    .find(&ApplicationContext::empty(), &specification)
                    .map_or_else(
                        |error| Response::Error(helpers::map_error(&error, correlation_id)),
                        |result| {
                            Response::Find(scriberig_protocol::FindResponse {
                                result: serde_json::to_value(result).unwrap_or_default(),
                            })
                        },
                    )
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
                }),
                Some((&request.session, &request.branch)),
                correlation_id,
            ),
            Request::SessionRead(request) => self.session_read(&request, correlation_id),
            Request::Project(_) => Response::Error(ProtocolError {
                code: ProtocolErrorCode::UnsupportedOperation,
                message: "project operations require the HTTP project catalog".to_owned(),
                details: None,
                correlation_id: Some(correlation_id.to_owned()),
            }),
        }
    }

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

    /// Reads one durable session through the application boundary.
    ///
    /// # Parameters
    /// - `request`: Typed session read request.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// Typed session response or protocol error.
    fn session_read(
        &self,
        request: &scriberig_protocol::SessionReadRequest,
        correlation_id: &str,
    ) -> Response {
        self.service
            .read_session(&command_context("session_branch_tree"), &request.session.id)
            .map_or_else(
                |error| Response::Error(helpers::map_error(&error, correlation_id)),
                |response| {
                    Response::Session(request.branch.as_ref().map_or(response.clone(), |branch| {
                        helpers::select_branch_subtree(response, &branch.id)
                    }))
                },
            )
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

    /// Maps one protocol inspection request to the shared application request.
    ///
    /// # Parameters
    /// - `request`: Protocol inspection request.
    /// - `correlation_id`: Request correlation identity.
    ///
    /// # Returns
    /// Typed protocol inspection response.
    fn inspect(
        &self,
        request: scriberig_protocol::InspectEntityRequest,
        correlation_id: &str,
    ) -> Response {
        let Ok(entity) = EntityId::from_str(&request.entity.id) else {
            return Response::Error(ProtocolError {
                code: ProtocolErrorCode::SchemaViolation,
                message: "invalid entity identifier".to_owned(),
                details: None,
                correlation_id: Some(correlation_id.to_owned()),
            });
        };
        let snapshot = match request.snapshot {
            scriberig_protocol::SnapshotSelector::Current
            | scriberig_protocol::SnapshotSelector::NamedRoot(_) => SnapshotResolution::Current,
            scriberig_protocol::SnapshotSelector::Exact(reference) => {
                match SnapshotId::from_str(&reference.id) {
                    Ok(value) => SnapshotResolution::At(value),
                    Err(_) => {
                        return Response::Error(ProtocolError {
                            code: ProtocolErrorCode::SchemaViolation,
                            message: "invalid snapshot identifier".to_owned(),
                            details: None,
                            correlation_id: Some(correlation_id.to_owned()),
                        });
                    }
                }
            }
        };
        let result = self.service.inspect(
            &ApplicationContext::empty(),
            &InspectRequest {
                entity,
                snapshot,
                detail: match request.detail {
                    scriberig_protocol::request::DetailLevel::Compact => DetailLevel::Compact,
                    scriberig_protocol::request::DetailLevel::Deep => DetailLevel::Detailed,
                },
            },
        );
        result.map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            |result| Response::Inspect(helpers::map_inspect(result)),
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
