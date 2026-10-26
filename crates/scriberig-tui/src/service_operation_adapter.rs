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
#[path = "service_operation_adapter/recovery.rs"]
mod recovery;
#[path = "service_operation_adapter/runtime.rs"]
mod runtime;

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

/// Requires a capability granted only by the trusted embedded TUI composition.
///
/// - `context`: Trusted application context authorizing configuration edits.
///
/// # Returns
/// Success when local upstream model configuration edits are authorized.
///
/// # Errors
/// Returns a permission or internal error when the capability is unavailable.
fn require_local_model_config_edit(context: &ApplicationContext) -> Result<(), ApplicationError> {
    let capability = scriberig_app::CapabilityId::new("agent_model_config_edit").map_err(|_| {
        ApplicationError::Internal {
            message: "model configuration capability is invalid".to_owned(),
        }
    })?;
    if context.capabilities.allows(&capability) {
        Ok(())
    } else {
        Err(ApplicationError::PermissionDenied {
            action: "edit upstream model properties".to_owned(),
            scope: "local model configuration".to_owned(),
        })
    }
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

    /// Returns the runner's active upstream component registry.
    /// # Returns
    /// The active component mailbox when configured.
    fn extension_component_mailbox(&self) -> Option<rpi_extensions::ExtensionStatusMailbox> {
        self.service.extension_component_mailbox()
    }

    /// Returns plugin slash commands admitted by the local runtime runner.
    /// # Returns
    /// The active plugin command descriptors.
    fn plugin_commands(&self) -> Vec<rpi_extensions::isolated::WorkerCommandSpec> {
        self.agent_runner.plugin_commands()
    }

    /// Invokes a plugin slash command through the host-authorized runner.
    ///
    /// - `command`: Exact normalized plugin command identity.
    /// - `arguments`: Raw text following the command name.
    ///
    /// # Returns
    /// The plugin's structured command response.
    ///
    /// # Errors
    /// Returns a safe failure when the host runner rejects the command.
    fn invoke_plugin_command(
        &self,
        command: &str,
        arguments: &str,
    ) -> Result<serde_json::Value, String> {
        self.agent_runner.invoke_plugin_command(command, arguments)
    }

    /// Continues a plugin command with a structured action from its response UI.
    ///
    /// - `command`: Exact normalized command identity.
    /// - `arguments`: Raw arguments from the initial invocation.
    /// - `action`: Structured plugin UI submission.
    ///
    /// # Returns
    /// The next structured plugin UI response.
    ///
    /// # Errors
    /// Returns the injected agent runner's safe failure.
    fn invoke_plugin_command_action(
        &self,
        command: &str,
        arguments: &str,
        action: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.agent_runner
            .invoke_plugin_command_action(command, arguments, action)
    }

    /// Reads upstream property descriptors for one locally authorized model.
    ///
    /// - `context`: Local authorization context for model settings.
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    ///
    /// # Returns
    /// Upstream schema-backed descriptors for the selected model.
    ///
    /// # Errors
    /// Returns an authorization or configuration error.
    fn model_properties(
        &self,
        context: &ApplicationContext,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<rpi_model_runtime::ModelPropertyDescriptor>, ApplicationError> {
        require_local_model_config_edit(context)?;
        self.agent_runner.model_properties(provider_id, model_id)
    }

    /// Updates a provider property only through upstream schema validation.
    ///
    /// - `context`: Local authorization context for model settings.
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    /// - `key`: Upstream schema property key.
    /// - `value`: User value validated by upstream.
    ///
    /// # Returns
    /// Success after the upstream config accepts the value.
    ///
    /// # Errors
    /// Returns an authorization, validation, or persistence error.
    fn set_model_property(
        &self,
        context: &ApplicationContext,
        provider_id: &str,
        model_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), ApplicationError> {
        require_local_model_config_edit(context)?;
        self.agent_runner
            .set_model_property(provider_id, model_id, key, value)
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

    tui_operation_contract_methods!();
}
