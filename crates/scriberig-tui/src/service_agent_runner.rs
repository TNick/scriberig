//! Defines the injected agent execution port used by the native TUI adapter.

use crate::service::AgentHandle;
use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService, LocalAgentHandle, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;

/// Executes an accepted agent request through the composed product runtime.
pub trait AgentCommandRunner<B: ApplicationBackend>: Send + Sync {
    /// Reads provider-schema-backed model settings from the local runtime.
    ///
    /// This host-only control does not expose model schemas over the public
    /// protocol. Implementations without a writable local model config return
    /// an unsupported-operation error.
    ///
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    /// - `_provider_id`: Exact upstream provider identity.
    /// - `_model_id`: Provider-native model identity.
    ///
    /// Validated upstream descriptors safe for the local property editor.
    ///
    /// # Returns
    /// The property descriptors available for this configured model.
    /// # Errors
    /// Returns an error when the local catalog or config is unavailable.
    fn model_properties(
        &self,
        _provider_id: &str,
        _model_id: &str,
    ) -> Result<Vec<rpi_model_runtime::ModelPropertyDescriptor>, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model property settings".to_owned(),
        })
    }

    /// Validates and applies one provider-schema-backed model setting.
    ///
    /// The upstream model runtime remains the schema and coercion authority.
    /// Implementations without a writable local model config return an
    /// unsupported-operation error.
    ///
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    /// - key: Provider-declared property key.
    /// - value: User value passed unchanged to upstream validation.
    /// - `_provider_id`: Exact upstream provider identity.
    /// - `_model_id`: Provider-native model identity.
    /// - `_key`: Provider-declared property key.
    /// - `_value`: User value passed unchanged to upstream validation.
    ///
    /// # Returns
    /// Success after the config and live catalog both accept the value.
    /// # Errors
    /// Returns an error when validation or config update fails.
    fn set_model_property(
        &self,
        _provider_id: &str,
        _model_id: &str,
        _key: &str,
        _value: serde_json::Value,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model property settings".to_owned(),
        })
    }

    /// Reports credential readiness for the configured provider models.
    ///
    /// Product runtimes may resolve readiness through their host-authorized
    /// credential boundary. The result contains no secret material.
    ///
    /// - `self`: Runtime adapter owning provider configuration.
    ///
    /// Safe per-model credential status rows, or an empty set when unavailable.
    /// # Returns
    /// Credential readiness rows safe for the model controls.
    fn provider_readiness(&self) -> Vec<scriberig_protocol::ProviderReadiness> {
        Vec::new()
    }

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
            failure: None,
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
            failure: None,
        })
    }
}
