//! Dispatches accepted agent operations through the injected host runtime.

use super::{Dispatcher, helpers};
use crate::server::ApplicationService;
use scriberig_protocol::{OperationStatusResponse, Response};

/// Builds a host-runtime request from the application's accepted handles.
///
/// - `accepted`: Operation, session, and branch allocated by the application.
/// - `prompt`: User instruction retained by the protocol request.
/// - `selection`: Optional exact semantic scope accepted by the application.
///
/// Agent command that addresses the exact accepted branch.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an internal application error if accepted identities are malformed.
fn accepted_agent_command(
    accepted: &scriberig_protocol::AgentAcceptedResponse,
    prompt: String,
    selection: Option<scriberig_app::AgentSelection>,
) -> Result<scriberig_app::AgentCommandRequest, scriberig_app::ApplicationError> {
    let session =
        accepted
            .session
            .id
            .parse()
            .map_err(|error| scriberig_app::ApplicationError::Internal {
                message: format!("accepted session identity is invalid: {error}"),
            })?;
    let branch = accepted
        .branch
        .as_ref()
        .ok_or_else(|| scriberig_app::ApplicationError::Internal {
            message: "accepted agent operation has no branch identity".to_owned(),
        })?
        .id
        .parse()
        .map_err(|error| scriberig_app::ApplicationError::Internal {
            message: format!("accepted branch identity is invalid: {error}"),
        })?;
    Ok(scriberig_app::AgentCommandRequest {
        session,
        branch: Some(branch),
        anchor: None,
        prompt,
        selection,
    })
}

impl<S: ApplicationService> Dispatcher<S> {
    /// Dispatches a new-session request through the accepted-operation runtime.
    ///
    /// - `request`: Initial prompt and optional semantic scope.
    /// - `handler`: Product runtime used to execute the accepted branch.
    /// - `correlation_id`: Protocol request identity.
    ///
    /// Accepted session handles or a structured protocol error.
    /// # Returns
    /// The operation result.
    pub(super) fn agent_new_session(
        &self,
        request: &scriberig_protocol::AgentNewSessionRequest,
        handler: Option<&crate::http::AgentExecutionHandler<S>>,
        correlation_id: &str,
    ) -> Response {
        let context = helpers::command_context("agent_start");
        let accepted = self
            .service
            .agent_new_session(&context, request)
            .and_then(|accepted| {
                self.execute_accepted_agent(
                    &context,
                    accepted,
                    request.prompt.clone(),
                    crate::server::server_session_operations::selection(
                        request.selection.as_ref(),
                    )?,
                    handler,
                )
            });
        accepted.map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::AgentAccepted,
        )
    }

    /// Dispatches an exact-anchor continuation through the accepted runtime.
    ///
    /// - `request`: Exact session anchor, prompt, and optional semantic scope.
    /// - `handler`: Product runtime used to execute the accepted branch.
    /// - `correlation_id`: Protocol request identity.
    ///
    /// Accepted continuation handles or a structured protocol error.
    /// # Returns
    /// The operation result.
    pub(super) fn agent_continue_at(
        &self,
        request: &scriberig_protocol::AgentContinueAtRequest,
        handler: Option<&crate::http::AgentExecutionHandler<S>>,
        correlation_id: &str,
    ) -> Response {
        let context = helpers::command_context("agent_continue");
        let accepted = self
            .service
            .agent_continue_at(&context, request)
            .and_then(|accepted| {
                self.execute_accepted_agent(
                    &context,
                    accepted,
                    request.prompt.clone(),
                    crate::server::server_session_operations::selection(
                        request.selection.as_ref(),
                    )?,
                    handler,
                )
            });
        accepted.map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            Response::AgentAccepted,
        )
    }

    /// Dispatches an existing-session command through the shared operation boundary.
    ///
    /// - `kind`: Stable agent operation identifier.
    /// - `payload`: Provider-neutral request payload.
    /// - `references`: Session and optional branch references for the response.
    /// - `handler`: Optional product-composed execution runtime.
    /// - `correlation_id`: Protocol request identity.
    ///
    /// Typed accepted agent response or protocol error.
    /// # Returns
    /// The operation result.
    pub(super) fn agent_command(
        &self,
        kind: &str,
        payload: &serde_json::Value,
        references: Option<(
            &scriberig_protocol::SessionReference,
            &scriberig_protocol::BranchReference,
        )>,
        handler: Option<&crate::http::AgentExecutionHandler<S>>,
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
        let context = helpers::command_context(kind);
        let command = handler.map_or_else(
            || self.service.command(&context, kind, payload),
            |handler| {
                crate::server::server_agent_command::decode_agent_command(kind, payload)
                    .and_then(|request| handler(&self.service, &context, &request, None))
            },
        );
        command.map_or_else(
            |error| Response::Error(helpers::map_error(&error, correlation_id)),
            |operation| {
                let branch = references.map(|(_, branch)| branch.clone()).or_else(|| {
                    self.service
                        .read_session(&helpers::command_context("session_branch_tree"), &session)
                        .ok()
                        .and_then(|value| value.branches.last().cloned())
                        .map(|branch| scriberig_protocol::BranchReference { id: branch.id })
                });
                Response::AgentAccepted(scriberig_protocol::AgentAcceptedResponse {
                    operation: OperationStatusResponse { operation },
                    session: scriberig_protocol::SessionReference { id: session },
                    branch,
                })
            },
        )
    }

    /// Runs the host runtime for an operation already accepted by the application.
    ///
    /// - `context`: Trusted command context for this request.
    /// - `accepted`: Durable operation, session, and branch handles.
    /// - `prompt`: User instruction retained by the protocol request.
    /// - `selection`: Exact semantic scope accepted by the application.
    /// - `handler`: Optional product-composed execution runtime.
    ///
    /// Updated accepted response containing the runtime's final operation state.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns a structured application error when execution fails.
    fn execute_accepted_agent(
        &self,
        context: &scriberig_app::ApplicationContext,
        mut accepted: scriberig_protocol::AgentAcceptedResponse,
        prompt: String,
        selection: Option<scriberig_app::AgentSelection>,
        handler: Option<&crate::http::AgentExecutionHandler<S>>,
    ) -> Result<scriberig_protocol::AgentAcceptedResponse, scriberig_app::ApplicationError> {
        if let Some(handler) = handler {
            let command = accepted_agent_command(&accepted, prompt, selection)?;
            accepted.operation.operation =
                handler(&self.service, context, &command, Some(&accepted))?;
        }
        Ok(accepted)
    }
}
