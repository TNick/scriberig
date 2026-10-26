//! Converts a server-accepted protocol operation into the native runner handle.

/// Builds a native accepted handle without beginning a second application operation.
///
/// - `request`: User request retained by the trusted server boundary.
/// - `accepted`: Stable operation, session, and branch allocated by the application.
///
/// Native runtime handle for the already accepted operation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an application error when an accepted identity is malformed or missing.
pub(crate) fn from_protocol_acceptance(
    request: &scriberig_app::AgentCommandRequest,
    accepted: &scriberig_protocol::AgentAcceptedResponse,
) -> Result<scriberig_app::LocalAgentHandle, scriberig_app::ApplicationError> {
    let operation = accepted
        .operation
        .operation
        .reference
        .id
        .parse()
        .map_err(|error| scriberig_app::ApplicationError::Internal {
            message: format!("accepted operation identity is invalid: {error}"),
        })?;
    let session =
        accepted
            .session
            .id
            .parse()
            .map_err(|error| scriberig_app::ApplicationError::Internal {
                message: format!("accepted session identity is invalid: {error}"),
            })?;
    let root_branch = accepted
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
    Ok(scriberig_app::LocalAgentHandle {
        operation,
        session,
        root_branch,
        prompt: request.prompt.clone(),
        selection: request.selection.clone(),
    })
}
