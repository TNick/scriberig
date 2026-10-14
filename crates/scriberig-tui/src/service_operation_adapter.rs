//! Adapts application operations to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Optional local project labels keyed by stable entity identity.
pub type ExplorerLabels = std::collections::HashMap<EntityId, String>;

/// Adapts the application backend to operation-oriented TUI services.
pub struct ApplicationOperationAdapter<B> {
    pub(crate) service: ApplicationService<B>,
    pub(crate) explorer_labels: ExplorerLabels,
    pub(crate) project_projection: Option<ProjectProjection>,
    /// Shared durable project workflow available even before project selection.
    pub(crate) project_workflow: Option<scriberig_project::ProjectService>,
    pub(crate) project_candidates: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, scriberig_project::ProjectCandidate>>,
    >,
    /// Last local lifecycle event sequence observed by this TUI adapter.
    pub(crate) event_cursor: std::sync::Mutex<Option<u64>>,
    pub(crate) agent_runner: std::sync::Arc<dyn super::AgentCommandRunner<B>>,
}

/// Adapts the public protocol client to the TUI inspection boundary.
impl<B: ApplicationBackend> ApplicationOperationAdapter<B> {
    /// Creates an operation adapter around an application service.
    ///
    /// # Parameters
    /// - `service`: Application service used by the adapter.
    ///
    /// # Returns
    /// An adapter with no project-specific explorer labels.
    #[must_use]
    pub fn new(service: ApplicationService<B>) -> Self {
        Self {
            service,
            explorer_labels: std::collections::HashMap::new(),
            project_projection: None,
            project_workflow: None,
            project_candidates: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            event_cursor: std::sync::Mutex::new(None),
            agent_runner: std::sync::Arc::new(super::SharedAgentCommandRunner),
        }
    }

    /// Supplies the runtime that executes agent commands for this TUI client.
    ///
    /// # Parameters
    /// - `self`: Adapter receiving the runtime.
    /// - `runner`: Product-composed provider and orchestration adapter.
    ///
    /// # Returns
    /// The configured application operation adapter.
    #[must_use]
    pub fn with_agent_runner<R>(mut self, runner: R) -> Self
    where
        R: super::AgentCommandRunner<B> + 'static,
    {
        self.agent_runner = std::sync::Arc::new(runner);
        self
    }

    /// Supplies project-relative labels for local explorer navigation.
    ///
    /// # Parameters
    /// - `self`: Adapter receiving the project labels.
    /// - `labels`: Human-readable labels keyed by stable entity identity.
    ///
    /// # Returns
    /// The adapter configured with the supplied labels.
    #[must_use]
    pub fn with_explorer_labels(mut self, labels: ExplorerLabels) -> Self {
        self.explorer_labels = labels;
        self
    }

    /// Supplies the active project's exact source projection data.
    ///
    /// # Parameters
    /// - `self`: Adapter receiving project projection data.
    /// - `projection`: Snapshot-bound source projection.
    ///
    /// # Returns
    /// The configured adapter.
    #[must_use]
    pub fn with_project_projection(mut self, projection: ProjectProjection) -> Self {
        self.project_workflow = Some(projection.workflow.clone());
        self.project_projection = Some(projection);
        self
    }

    /// Supplies durable project operations for candidates created after onboarding.
    ///
    /// # Parameters
    /// - `self`: Adapter receiving the shared project workflow.
    /// - `workflow`: Project service using the product's durable store.
    ///
    /// # Returns
    /// The configured application operation adapter.
    #[must_use]
    pub fn with_project_workflow(mut self, workflow: scriberig_project::ProjectService) -> Self {
        self.project_workflow = Some(workflow);
        self
    }
}

impl<B: ApplicationBackend> TuiService for ApplicationOperationAdapter<B> {
    /// Submits local agent work through the runtime composed by the product.
    ///
    /// # Parameters
    /// - `self`: Adapter with the injected shared runtime boundary.
    /// - `context`: Application context for the operation.
    /// - `session`: Stable session identity.
    /// - `branch`: Optional continuation branch.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and branch handles.
    ///
    /// # Errors
    /// Returns an application submission or session-read error.
    fn submit_agent_handle(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<super::AgentHandle, ApplicationError> {
        self.agent_runner
            .submit(&self.service, context, session, branch, prompt)
    }

    /// Reports the semantic operations exposed by the embedded application.
    ///
    /// # Parameters
    /// - `self`: Embedded application adapter.
    /// - `_context`: Authorization context used by the surrounding runtime.
    ///
    /// # Returns
    /// Capability descriptors used to keep the command palette honest.
    ///
    /// # Errors
    /// This adapter has no separate capability-discovery failure path.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        service_operation_helpers::runtime_capabilities(self)
    }

    /// Runs the deterministic provider through the selected semantic scope.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `entity`: Selected semantic entity.
    /// - `snapshot`: Exact candidate base snapshot.
    /// - `prompt`: User instruction.
    ///
    /// # Returns
    /// The normal candidate review operation result.
    ///
    /// # Errors
    /// Returns a structured agent, validation, or storage error.
    fn project_agent(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: SnapshotId,
        prompt: &str,
    ) -> Result<ApplicationResult, ApplicationError> {
        service_operation_agent::project_agent(self, context, entity, snapshot, prompt)
    }

    /// Delegates inspection through the public application facade.
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
    /// Returns the application inspection error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        service_operation_helpers::inspect_entity(self, context, entity, snapshot)
    }

    /// Lists the shared snapshot entity collection for Explorer navigation.
    ///
    /// # Parameters
    /// - `context`: Application context.
    /// - `entity`: Entity owning the collection.
    /// - `snapshot`: Exact snapshot context.
    /// - `continuation`: Optional continuation token.
    ///
    /// # Returns
    /// A bounded Explorer page.
    ///
    /// # Errors
    /// Returns the application listing error.
    fn list_explorer(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: SnapshotId,
        continuation: Option<scriberig_query::ContinuationToken>,
    ) -> Result<crate::explorer::ExplorerPage, ApplicationError> {
        service_operation_helpers::list_explorer(self, context, entity, snapshot, continuation)
    }

    /// Executes exact-text discovery through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Application context.
    /// - `snapshot`: Exact discovery scope.
    /// - `query`: Search text.
    ///
    /// # Returns
    /// A bounded find model.
    ///
    /// # Errors
    /// Returns the application discovery error.
    fn find_entities(
        &self,
        context: &ApplicationContext,
        snapshot: SnapshotId,
        query: &str,
    ) -> Result<crate::view::find::FindModel, ApplicationError> {
        service_operation_helpers::find_entities(self, context, snapshot, query)
    }

    /// Delegates shared operations through the public application facade.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Public operation request.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns the application operation error.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        if let Some(result) = service_operation_helpers::execute_operation(self, context, request)?
        {
            return Ok(result);
        }
        self.service.execute(context, request)
    }

    /// Reads an operation record from the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The current operation record.
    ///
    /// # Errors
    /// Returns a lookup error when the operation is unknown.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        service_operation_helpers::operation_status(self, operation)
    }

    /// Cancels an operation through the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for the cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled operation record.
    ///
    /// # Errors
    /// Returns a cancellation error when the operation is unknown.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        service_operation_helpers::cancel_operation(self, operation)
    }

    /// Lists current shared application operation records.
    ///
    /// # Parameters
    /// - `_context`: Application context for operation visibility.
    ///
    /// # Returns
    /// Current application operation records.
    ///
    /// # Errors
    /// This local adapter has no separate listing failure path.
    fn operation_records(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::OperationRecord>, ApplicationError> {
        Ok(self.service.operations().list())
    }

    /// Returns lifecycle events retained by the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Retained application events.
    ///
    /// # Errors
    /// This adapter does not return an event access error.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        Ok(service_operation_helpers::application_events(self))
    }

    /// Reads a session through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `session`: Session identity to read.
    ///
    /// # Returns
    /// The requested session.
    ///
    /// # Errors
    /// Returns the application session error.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        service_operation_helpers::read_session(self, context, session)
    }

    /// Submits an agent command through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `session`: Session identity receiving the prompt.
    /// - `branch`: Optional branch identity.
    /// - `prompt`: Agent prompt to submit.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns the application agent error.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        service_operation_helpers::submit_agent_command(self, context, session, branch, prompt)
    }
}
