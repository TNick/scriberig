//! Adapts exact session operations through the shared application layer.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService as SharedService,
    OperationRecord,
};

/// Adapts a new-session request to application allocated session handles.
///
/// - `service`: Shared application service.
/// - `context`: Trusted caller capability context.
/// - `request`: New-session prompt and semantic selection.
///
/// Accepted operation with authoritative session and root branch references.
///
/// Returns authorization, selection, or persistence failures.
pub(super) fn start_new<B: ApplicationBackend>(
    service: &SharedService<B>,
    context: &ApplicationContext,
    request: &scriberig_protocol::AgentNewSessionRequest,
) -> Result<scriberig_protocol::AgentAcceptedResponse, ApplicationError> {
    let handle = service.begin_new_session_agent(
        context,
        scriberig_app::NewSessionAgentRequest {
            prompt: request.prompt.clone(),
            selection: selection(request.selection.as_ref())?,
        },
    )?;
    accepted(
        service,
        &handle.operation,
        handle.session,
        Some(handle.root_branch),
    )
}

/// Adapts an exact continuation request to shared application semantics.
///
/// - `service`: Shared application service.
/// - `context`: Trusted caller capability context.
/// - `request`: Exact anchor, prompt, and semantic selection.
///
/// Accepted operation with authoritative session and child branch references.
///
/// Returns authorization, anchor, selection, or persistence failures.
pub(super) fn continue_at<B: ApplicationBackend>(
    service: &SharedService<B>,
    context: &ApplicationContext,
    request: &scriberig_protocol::AgentContinueAtRequest,
) -> Result<scriberig_protocol::AgentAcceptedResponse, ApplicationError> {
    let handle = service.begin_continue_agent_at(
        context,
        scriberig_app::AgentContinueAtRequest {
            session: parse_entity(&request.session.id, "agent continue at anchor")?,
            anchor: anchor(&request.anchor)?,
            prompt: request.prompt.clone(),
            selection: selection(request.selection.as_ref())?,
        },
    )?;
    accepted(
        service,
        &handle.operation,
        handle.session,
        Some(handle.root_branch),
    )
}

/// Builds a protocol acceptance from stable application handles.
///
/// - `service`: Shared application service owning operation state.
/// - `operation`: Accepted operation identity.
/// - `session`: Authoritative session identity.
/// - `branch`: Root or continuation branch identity.
///
/// Safe accepted response.
///
/// Returns internal error when an accepted operation has no durable record.
fn accepted<B: ApplicationBackend>(
    service: &SharedService<B>,
    operation: &scriberig_core::OperationId,
    session: scriberig_core::EntityId,
    branch: Option<scriberig_core::EntityId>,
) -> Result<scriberig_protocol::AgentAcceptedResponse, ApplicationError> {
    let record = service.operations().get(operation)?;
    Ok(scriberig_protocol::AgentAcceptedResponse {
        operation: scriberig_protocol::OperationStatusResponse {
            operation: operation_record(record),
        },
        session: scriberig_protocol::SessionReference {
            id: session.to_string(),
        },
        branch: branch.map(|branch| scriberig_protocol::BranchReference {
            id: branch.to_string(),
        }),
    })
}

/// Converts protocol semantic scope without introducing credentials.
///
/// - `value`: Optional safe semantic selection.
///
/// Shared application selection when supplied.
///
/// Returns invalid-request for malformed semantic identities.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn selection(
    value: Option<&scriberig_protocol::AgentSelection>,
) -> Result<Option<scriberig_app::AgentSelection>, ApplicationError> {
    value
        .map(|selection| {
            Ok(scriberig_app::AgentSelection {
                project: selection
                    .project_id
                    .as_deref()
                    .map(str::parse)
                    .transpose()
                    .map_err(|error| invalid_identity("selected project", error))?,
                root: selection
                    .root
                    .as_ref()
                    .map(|root| parse_entity(&root.id, "agent selection"))
                    .transpose()?
                    .unwrap_or(parse_entity(&selection.entity.id, "agent selection")?),
                entity: parse_entity(&selection.entity.id, "agent selection")?,
                snapshot: selection
                    .snapshot
                    .id
                    .parse()
                    .map_err(|error| invalid_identity("selected snapshot", error))?,
            })
        })
        .transpose()
}

/// Converts an exact protocol anchor to its session-domain identity.
///
/// - `value`: Stable protocol continuation anchor.
///
/// Shared exact continuation anchor.
///
/// Returns invalid-request for malformed entity identities.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn anchor(
    value: &scriberig_protocol::SessionContinuationAnchor,
) -> Result<scriberig_session::ContinuationAnchor, ApplicationError> {
    Ok(match value {
        scriberig_protocol::SessionContinuationAnchor::BranchHead { branch } => {
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: parse_entity(branch, "agent continue at anchor")?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::AssistantOutput { branch, turn } => {
            scriberig_session::ContinuationAnchor::AssistantOutput {
                branch: parse_entity(branch, "agent continue at anchor")?,
                turn: parse_entity(turn, "agent continue at anchor")?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::HumanInput { branch, turn } => {
            scriberig_session::ContinuationAnchor::HumanInput {
                branch: parse_entity(branch, "agent continue at anchor")?,
                turn: parse_entity(turn, "agent continue at anchor")?,
            }
        }
        scriberig_protocol::SessionContinuationAnchor::BranchResult { branch, result } => {
            scriberig_session::ContinuationAnchor::BranchResult {
                branch: parse_entity(branch, "agent continue at anchor")?,
                result: parse_entity(result, "agent continue at anchor")?,
            }
        }
    })
}

/// Parses a stable entity identity for an application request.
///
/// - `value`: Serialized entity identity.
/// - `operation`: Safe operation label for errors.
///
/// Typed entity identity.
///
/// Returns invalid-request when the identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_entity(
    value: &str,
    operation: &str,
) -> Result<scriberig_core::EntityId, ApplicationError> {
    value
        .parse()
        .map_err(|error| invalid_identity(operation, error))
}

/// Converts identity parser errors to safe invalid-request failures.
///
/// - `operation`: Operation being validated.
/// - `error`: Identity parser failure.
///
/// Structured invalid-request error.
/// # Returns
/// The operation result.
fn invalid_identity(operation: &str, error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: operation.to_owned(),
        message: format!("invalid stable identity: {error}"),
    }
}

/// Converts a shared operation record to its safe protocol shape.
///
/// - `record`: Shared operation lifecycle state.
///
/// Protocol operation summary.
/// # Returns
/// The operation result.
fn operation_record(record: OperationRecord) -> scriberig_protocol::Operation {
    super::server_helpers::operation_from_record(record)
}

macro_rules! shared_session_operation_methods {
    () => {
        /// Starts a new session through the shared session service.
        /// # Parameters
        /// - `context`: Input for this operation.
        /// - `request`: Input for this operation.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn agent_new_session(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::AgentNewSessionRequest,
        ) -> Result<scriberig_protocol::AgentAcceptedResponse, ApplicationError> {
            super::server_session_operations::start_new(self, context, request)
        }

        /// Continues from one exact shared conversation anchor.
        /// # Parameters
        /// - `context`: Input for this operation.
        /// - `request`: Input for this operation.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn agent_continue_at(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::AgentContinueAtRequest,
        ) -> Result<scriberig_protocol::AgentAcceptedResponse, ApplicationError> {
            super::server_session_operations::continue_at(self, context, request)
        }
    };
}
