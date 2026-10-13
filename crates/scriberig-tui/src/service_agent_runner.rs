//! Defines the injected agent execution port used by the native TUI adapter.

use crate::service::AgentHandle;
use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;

/// Executes an accepted agent request through the composed product runtime.
pub trait AgentCommandRunner<B: ApplicationBackend>: Send + Sync {
    /// Submits agent work and returns its durable application handles.
    ///
    /// # Parameters
    /// - `self`: Runtime adapter selected by the product composition.
    /// - `service`: Shared application service owning agent/session state.
    /// - `context`: Trusted caller capabilities and semantic scope.
    /// - `session`: Session receiving the user instruction.
    /// - `branch`: Optional branch receiving the instruction.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and branch handles.
    ///
    /// # Errors
    /// Returns an application failure when submission or execution fails.
    fn submit(
        &self,
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<AgentHandle, ApplicationError>;
}

/// Uses the shared application session contract without adding a provider.
#[derive(Clone, Copy, Debug, Default)]
pub struct SharedAgentCommandRunner;

impl<B: ApplicationBackend> AgentCommandRunner<B> for SharedAgentCommandRunner {
    /// Accepts agent work through the shared session contract.
    ///
    /// # Parameters
    /// - `self`: Provider-free fallback adapter.
    /// - `service`: Shared application service owning agent/session state.
    /// - `context`: Trusted caller capabilities and semantic scope.
    /// - `session`: Session receiving the user instruction.
    /// - `branch`: Optional branch receiving the instruction.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and branch handles.
    ///
    /// # Errors
    /// Returns an application failure when the shared operation is rejected.
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
        })
    }
}
