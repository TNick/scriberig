//! Adapts application operations to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::*;
#[macro_use]
#[path = "service_operation_contract_methods.rs"]
mod operation_contract_methods;
#[path = "service_operation_adapter/accepted_agent.rs"]
mod accepted_agent;
#[macro_use]
#[path = "service_operation_adapter/agent_methods.rs"]
mod agent_methods;

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
    /// - `service`: Application service used by the adapter.
    ///
    /// An adapter with no project-specific explorer labels.
    #[must_use]
    /// # Returns
    /// The operation result.
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
    /// - `self`: Adapter receiving the runtime.
    /// - `runner`: Product-composed provider and orchestration adapter.
    ///
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
    /// - `self`: Adapter receiving the project labels.
    /// - `labels`: Human-readable labels keyed by stable entity identity.
    ///
    /// The adapter configured with the supplied labels.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_explorer_labels(mut self, labels: ExplorerLabels) -> Self {
        self.explorer_labels = labels;
        self
    }

    /// Supplies the active project's exact source projection data.
    ///
    /// - `self`: Adapter receiving project projection data.
    /// - `projection`: Snapshot-bound source projection.
    ///
    /// The configured adapter.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_project_projection(mut self, projection: ProjectProjection) -> Self {
        self.project_workflow = Some(projection.workflow.clone());
        self.project_projection = Some(projection);
        self
    }

    /// Supplies durable project operations for candidates created after onboarding.
    ///
    /// - `self`: Adapter receiving the shared project workflow.
    /// - `workflow`: Project service using the product's durable store.
    ///
    /// The configured application operation adapter.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_project_workflow(mut self, workflow: scriberig_project::ProjectService) -> Self {
        self.project_workflow = Some(workflow);
        self
    }
}

impl<B: ApplicationBackend> TuiService for ApplicationOperationAdapter<B> {
    operation_agent_methods!();

    /// Reports the semantic operations exposed by the embedded application.
    ///
    /// - `self`: Embedded application adapter.
    /// - `_context`: Authorization context used by the surrounding runtime.
    ///
    /// Capability descriptors used to keep the command palette honest.
    ///
    /// This adapter has no separate capability-discovery failure path.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        service_operation_helpers::runtime_capabilities(self)
    }

    /// Runs the deterministic provider through the selected semantic scope.
    ///
    /// - `context`: Trusted application context.
    /// - `entity`: Selected semantic entity.
    /// - `snapshot`: Exact candidate base snapshot.
    /// - `prompt`: User instruction.
    ///
    /// The normal candidate review operation result.
    ///
    /// Returns a structured agent, validation, or storage error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// The inspection result.
    ///
    /// Returns the application inspection error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `context`: Application context.
    /// - `entity`: Entity owning the collection.
    /// - `snapshot`: Exact snapshot context.
    /// - `continuation`: Optional continuation token.
    ///
    /// A bounded Explorer page.
    ///
    /// Returns the application listing error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `context`: Application context.
    /// - `snapshot`: Exact discovery scope.
    /// - `query`: Search text.
    ///
    /// A bounded find model.
    ///
    /// Returns the application discovery error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `context`: Application context for the request.
    /// - `request`: Public operation request.
    ///
    ///
    /// Returns the application operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// The current operation record.
    ///
    /// Returns a lookup error when the operation is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        service_operation_helpers::operation_status(self, operation)
    }

    /// Cancels an operation through the shared application registry.
    ///
    /// - `_context`: Application context for the cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// The cancelled operation record.
    ///
    /// Returns a cancellation error when the operation is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        service_operation_helpers::cancel_operation(self, operation)
    }

    tui_operation_contract_methods!();
}
