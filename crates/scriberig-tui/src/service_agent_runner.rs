//! Defines the injected agent execution port used by the native TUI adapter.

use crate::service::AgentHandle;
use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService, LocalAgentHandle, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;

/// Executes an accepted agent request through the composed product runtime.
pub trait AgentCommandRunner<B: ApplicationBackend>: Send + Sync {
    /// Applies a plan lifecycle transition and advances it when supported.
    ///
    /// Product runners may run the shared durable coordinator after accepting
    /// a start or resume transition. Provider-free adapters only persist the
    /// lifecycle change.
    ///
    /// - `self`: Runtime adapter selected by the product composition.
    /// - `service`: Shared application service owning plan state.
    /// - `context`: Trusted caller capabilities and scope.
    /// - `request`: Plan identity and observed revision.
    /// - `action`: Requested plan transition.
    ///
    /// The latest authoritative plan after lifecycle processing.
    ///
    /// Returns an application or worker execution failure.
    /// # Returns
    /// The authoritative plan.
    /// # Errors
    /// Returns an error when the transition or execution fails.
    fn plan_lifecycle(
        &self,
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        request: &scriberig_app::PlanLifecycleRequest,
        action: scriberig_app::PlanLifecycleAction,
    ) -> Result<scriberig_session::Plan, ApplicationError>
    where
        ApplicationService<B>: SessionBackend,
    {
        scriberig_app::SessionBackend::plan_lifecycle(service, context, request, action)
    }

    /// Submits agent work and returns its durable application handles.
    ///
    /// - `self`: Runtime adapter selected by the product composition.
    /// - `service`: Shared application service owning agent/session state.
    /// - `context`: Trusted caller capabilities and semantic scope.
    /// - `session`: Session receiving the user instruction.
    /// - `branch`: Optional branch receiving the instruction.
    /// - `prompt`: User-authored instruction.
    ///
    /// Stable operation, session, and branch handles.
    ///
    /// Returns an application failure when submission or execution fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit(
        &self,
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<AgentHandle, ApplicationError>;

    /// Executes an operation whose session turn and identity are already durable.
    ///
    /// Product runners should continue the accepted operation without creating
    /// another user turn. Provider-free adapters return the accepted handles.
    ///
    /// - `self`: Runtime adapter selected by the product composition.
    /// - `service`: Shared application service owning agent/session state.
    /// - `context`: Trusted caller capabilities and semantic scope.
    /// - `handle`: Durable operation, branch, and original user prompt.
    ///
    /// Stable operation, session, and branch handles after execution.
    ///
    /// Returns an application failure when execution fails.
    /// # Parameters
    /// - `_service`: Input for this operation.
    /// - `_context`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute_accepted(
        &self,
        _service: &ApplicationService<B>,
        _context: &ApplicationContext,
        handle: &LocalAgentHandle,
    ) -> Result<AgentHandle, ApplicationError> {
        Ok(AgentHandle {
            operation: handle.operation.clone(),
            session: handle.session,
            branch: Some(handle.root_branch),
            workspace: None,
        })
    }
}

/// Uses the shared application session contract without adding a provider.
#[derive(Clone, Copy, Debug, Default)]
pub struct SharedAgentCommandRunner;

impl<B: ApplicationBackend> AgentCommandRunner<B> for SharedAgentCommandRunner {
    /// Accepts agent work through the shared session contract.
    ///
    /// - `self`: Provider-free fallback adapter.
    /// - `service`: Shared application service owning agent/session state.
    /// - `context`: Trusted caller capabilities and semantic scope.
    /// - `session`: Session receiving the user instruction.
    /// - `branch`: Optional branch receiving the instruction.
    /// - `prompt`: User-authored instruction.
    ///
    /// Stable operation, session, and branch handles.
    ///
    /// Returns an application failure when the shared operation is rejected.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit(
        &self,
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<AgentHandle, ApplicationError> {
        let operation = SessionBackend::submit_agent_command(
            service,
            context,
            &AgentCommandRequest {
                session,
                branch,
                anchor: None,
                prompt: prompt.to_owned(),
                selection: None,
            },
        )?;
        let branch = branch.or_else(|| {
            SessionBackend::read_session(service, context, &SessionRequest { session })
                .ok()
                .and_then(|value| value.branches.last().map(|value| value.id))
        });
        Ok(AgentHandle {
            operation,
            session,
            branch,
            workspace: None,
        })
    }
}
