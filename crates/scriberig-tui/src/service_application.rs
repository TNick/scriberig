//! Adapts the application query service to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_app::SessionBackend;

/// Adapts an application query service to TUI operations.
pub struct ApplicationAdapter<S> {
    service: S,
    capabilities: Vec<RuntimeCapability>,
}

impl<S> ApplicationAdapter<S> {
    /// Creates a TUI adapter around a public application service.
    ///
    /// # Parameters
    /// - `service`: Public query service.
    /// - `capabilities`: Runtime capabilities exposed by the service.
    ///
    /// # Returns
    /// A configured TUI adapter.
    #[must_use]
    pub fn new(service: S, capabilities: Vec<RuntimeCapability>) -> Self {
        Self {
            service,
            capabilities,
        }
    }

    /// Borrows the wrapped application service.
    #[must_use]
    pub const fn service(&self) -> &S {
        &self.service
    }
}

impl<S: QueryService + SessionBackend> TuiService for ApplicationAdapter<S> {
    /// Delegates inspection through the public query service.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the query service error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.service.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }

    /// Returns the capabilities supplied by the composition root.
    ///
    /// # Parameters
    /// - `_context`: Application context for capability access.
    ///
    /// # Returns
    /// The configured runtime capabilities.
    ///
    /// # Errors
    /// This adapter does not return a capability error.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(self.capabilities.clone())
    }

    /// Reads a session through the public application boundary.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    /// - `session`: Stable session identity.
    ///
    /// # Returns
    /// The requested session snapshot.
    ///
    /// # Errors
    /// Returns the application session error.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        self.service
            .read_session(context, &scriberig_app::SessionRequest { session })
    }

    /// Submits an agent command through the public application boundary.
    ///
    /// # Parameters
    /// - `context`: Caller authorization context.
    /// - `session`: Target session identity.
    /// - `branch`: Optional target branch.
    /// - `prompt`: User-authored command text.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns the application session error.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        self.service.submit_agent_command(
            context,
            &scriberig_app::AgentCommandRequest {
                session,
                branch,
                prompt: prompt.to_owned(),
            },
        )
    }

    /// Submits local agent work and resolves the accepted root branch handle.
    ///
    /// # Parameters
    /// - `context`: Application context for the operation.
    /// - `session`: Stable session identity.
    /// - `branch`: Optional continuation branch.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and branch handles.
    ///
    /// # Errors
    /// Returns the application submission or session-read error.
    fn submit_agent_handle(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<super::AgentHandle, ApplicationError> {
        let operation = self.submit_agent_command(context, session, branch, prompt)?;
        let branch = branch.or_else(|| {
            self.read_session(context, session)
                .ok()
                .and_then(|value| value.branches.last().map(|value| value.id))
        });
        Ok(super::AgentHandle {
            operation,
            session,
            branch,
        })
    }
}

impl<S: QueryService> TuiService for S {
    /// Delegates inspection through a public query service.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the query service error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }
}
