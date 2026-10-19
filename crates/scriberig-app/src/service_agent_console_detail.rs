//! Reads bounded tool-result detail through the application session boundary.

use crate::session::ToolResultDetail;
use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, SessionBackend,
    SessionRequest,
};
use scriberig_core::EntityId;

/// Loads a durable result and projects only its allowlisted safe fields.
///
/// - `service`: Application service that owns session reads.
/// - `context`: Authorized caller context.
/// - `session`: Session containing the result.
/// - `branch`: Branch containing the result.
/// - `result`: Stable tool-result identity.
///
/// Bounded safe result detail with its stable identity.
///
/// Returns authorization, lookup, or persistence errors.
pub(super) fn tool_result_detail<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    result: EntityId,
) -> Result<ToolResultDetail, ApplicationError> {
    let session_data = SessionBackend::read_session(service, context, &SessionRequest { session })?;
    let branch_data = session_data
        .branches
        .iter()
        .find(|candidate| candidate.id == branch)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: branch.to_string(),
        })?;
    let tool_result = branch_data
        .tool_results
        .iter()
        .find(|candidate| candidate.id == result)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: result.to_string(),
        })?;
    let (detail, truncated) =
        crate::agent_console::detail::safe_tool_result_detail(&tool_result.output);
    Ok(ToolResultDetail {
        result,
        detail,
        truncated,
    })
}
