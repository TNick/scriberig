//! Adapts application operations to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::*;

use scriberig_agent_runtime::AgentRuntime;

/// Optional local project labels keyed by stable entity identity.
pub type ExplorerLabels = std::collections::HashMap<EntityId, String>;

/// Adapts the application backend to operation-oriented TUI services.
pub struct ApplicationOperationAdapter<B> {
    pub(crate) service: ApplicationService<B>,
    pub(crate) explorer_labels: ExplorerLabels,
    pub(crate) project_projection: Option<ProjectProjection>,
    pub(crate) project_candidates: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, scriberig_project::ProjectCandidate>>,
    >,
}

/// Adapts the public protocol client to the TUI inspection boundary.
impl<B> ApplicationOperationAdapter<B> {
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
            project_candidates: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
        }
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
        self.project_projection = Some(projection);
        self
    }
}

impl<B: ApplicationBackend> TuiService for ApplicationOperationAdapter<B> {
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
        let projection = self.project_projection.as_ref().ok_or_else(|| {
            ApplicationError::UnsupportedOperation {
                operation: "project agent".to_owned(),
            }
        })?;
        let (path, bytes) =
            projection
                .sources
                .get(&entity)
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: entity.to_string(),
                })?;
        let replacement =
            String::from_utf8_lossy(bytes).replace("left + right", "left.saturating_add(right)");
        let tool = ProjectAgentTool {
            projection,
            entity,
            snapshot,
        };
        let mut runtime = AgentRuntime::new(golden_provider(entity, snapshot), tool, 4);
        let result = runtime
            .run(context, prompt, &std::sync::atomic::AtomicBool::new(false))
            .map_err(|error| ApplicationError::OperationFailed {
                operation: "project_agent".to_owned(),
                message: error.to_string(),
            })?;
        let candidate = result
            .messages
            .iter()
            .rev()
            .find_map(|message| match message {
                scriberig_agent_runtime::AgentMessage::Tool(value)
                    if value.object_type.as_deref() == Some("migration_candidate") =>
                {
                    value.handles.first().cloned()
                }
                _ => None,
            })
            .ok_or_else(|| ApplicationError::OperationFailed {
                operation: "project_agent".to_owned(),
                message: "agent did not create a candidate".to_owned(),
            })?;
        let candidate_value = projection
            .workflow
            .propose_candidate(projection.project, entity, replacement)
            .map_err(|error| ApplicationError::ValidationFailure {
                message: error.to_string(),
            })?;
        if candidate != candidate_value.id {
            return Err(ApplicationError::Internal {
                message: "agent candidate identity changed between tool calls".to_owned(),
            });
        }
        self.project_candidates
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "project candidate lock poisoned".to_owned(),
            })?
            .insert(candidate_value.id.clone(), candidate_value.clone());
        let validation = match &candidate_value.validation {
            scriberig_project::CandidateValidation::Passed => {
                scriberig_app::ValidationStatus::Passed
            }
            scriberig_project::CandidateValidation::Failed { .. } => {
                scriberig_app::ValidationStatus::Failed
            }
        };
        Ok(ApplicationResult::Apply(scriberig_app::OperationSummary {
            handle: candidate_value.id,
            snapshot: Some(snapshot),
            revision: None,
            branch: None,
            diff: None,
            history: None,
            rendered: None,
            validation: Some(validation),
            validation_diagnostics: Vec::new(),
            conflicts: vec![format!(
                "scope: entity={entity} snapshot={snapshot}; {path}: add | old: left + right | new: left.saturating_add(right)"
            )],
        }))
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

    /// Submits local agent work and preserves the accepted branch identity.
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
        service_operation_helpers::submit_agent_handle(self, context, session, branch, prompt)
    }
}
