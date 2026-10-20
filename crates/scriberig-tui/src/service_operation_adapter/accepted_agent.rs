//! Retains accepted session handles when provider execution fails.

use super::*;

/// Executes accepted work and carries failures beside its durable handles.
///
/// - `runner`: Product-composed agent runtime.
/// - `service`: Shared application session service.
/// - `context`: Trusted capabilities for the invocation.
/// - `accepted`: Durable operation, session, branch, and prompt.
///
/// Stable accepted handles and an optional safe execution failure.
/// # Returns
/// The durable acceptance and any execution failure that followed it.
pub(super) fn execute<B: ApplicationBackend>(
    runner: &dyn AgentCommandRunner<B>,
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    accepted: &scriberig_app::LocalAgentHandle,
) -> AgentHandle {
    match runner.execute_accepted(service, context, accepted) {
        Ok(handle) => handle,
        Err(error) => AgentHandle {
            operation: accepted.operation.clone(),
            session: accepted.session,
            branch: Some(accepted.root_branch),
            workspace: None,
            failure: Some(AgentExecutionFailure {
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        },
    }
}
