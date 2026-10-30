//! Adapts the application query service to the TUI service contract.

#![allow(clippy::wildcard_imports)]

#[macro_use]
#[path = "service_application_handle_method.rs"]
mod handle_method;

#[path = "service_application_plan_mutation_methods.rs"]
#[macro_use]
mod plan_mutation_methods;

use super::*;
use scriberig_app::{AgentConsoleBackend, SessionBackend};

/// Adapts an application query service to TUI operations.
pub struct ApplicationAdapter<S> {
    pub(super) service: S,
    capabilities: Vec<RuntimeCapability>,
}

impl<S> ApplicationAdapter<S> {
    /// Creates a TUI adapter around a public application service.
    ///
    /// - `service`: Public query service.
    /// - `capabilities`: Runtime capabilities exposed by the service.
    ///
    /// A configured TUI adapter.
    #[must_use]
    /// # Returns
    /// The operation result.
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

impl<S: QueryService + SessionBackend + AgentConsoleBackend> TuiService for ApplicationAdapter<S> {
    /// Continues an exact anchor through the shared session backend.
    ///
    /// - `context`: Caller authorization context.
    /// - `session`: Owning session identity.
    /// - `anchor`: Exact replyable continuation location.
    /// - `prompt`: User-authored follow-up.
    ///
    /// Authoritative operation, session, and branch handles.
    ///
    /// Returns the shared application validation or persistence error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn continue_agent_at_handle(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        anchor: scriberig_session::ContinuationAnchor,
        prompt: &str,
    ) -> Result<super::AgentHandle, ApplicationError> {
        let accepted = self.service.continue_agent_at(
            context,
            scriberig_app::AgentContinueAtRequest {
                session,
                anchor,
                prompt: prompt.to_owned(),
                selection: None,
            },
        )?;
        Ok(super::AgentHandle {
            operation: accepted.operation,
            session: accepted.session,
            branch: Some(accepted.branch),
            workspace: None,
            failure: None,
        })
    }

    /// Starts a session through the shared application operation.
    ///
    /// - `context`: Caller authorization context.
    /// - `prompt`: User-authored first instruction.
    /// - `selection`: Exact semantic scope for the initial model context.
    ///
    /// Authoritative operation, session, and root branch handles.
    ///
    /// Returns the shared application validation or persistence error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn start_new_session_handle(
        &self,
        context: &ApplicationContext,
        prompt: &str,
        selection: Option<scriberig_app::AgentSelection>,
    ) -> Result<super::AgentHandle, ApplicationError> {
        let accepted = self.service.start_new_session_agent(
            context,
            scriberig_app::NewSessionAgentRequest {
                prompt: prompt.to_owned(),
                selection,
            },
        )?;
        Ok(super::AgentHandle {
            operation: accepted.operation,
            session: accepted.session,
            branch: Some(accepted.root_branch),
            workspace: None,
            failure: None,
        })
    }

    /// Delegates inspection through the public query service.
    ///
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// The inspection result.
    ///
    /// Returns the query service error.
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
    /// - `_context`: Application context for capability access.
    ///
    /// The configured runtime capabilities.
    ///
    /// This adapter does not return a capability error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(self.capabilities.clone())
    }

    /// Reads a session through the public application boundary.
    ///
    /// - `context`: Caller authorization context.
    /// - `session`: Stable session identity.
    ///
    /// The requested session snapshot.
    ///
    /// Returns the application session error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        self.service
            .read_session(context, &scriberig_app::SessionRequest { session })
    }

    /// Creates a structured plan using the shared application plan service.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Structured plan proposal at an exact conversation anchor.
    ///
    /// The persisted plan with application allocated identities.
    ///
    /// Returns application validation or persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_plan(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::PlanCreateRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        scriberig_app::SessionBackend::create_plan(&self.service, context, request)
    }

    /// Applies a durable plan revision through the application service.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Complete revision proposal.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns application validation, conflict, or persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn replan(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::PlanReplanRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        scriberig_app::SessionBackend::replan(&self.service, context, request)
    }

    /// Applies one durable plan lifecycle change through the shared service.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Plan identity and observed revision.
    /// - `action`: Requested plan transition.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns application validation, conflict, or persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn plan_lifecycle(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::PlanLifecycleRequest,
        action: scriberig_app::PlanLifecycleAction,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        scriberig_app::SessionBackend::plan_lifecycle(&self.service, context, request, action)
    }

    /// Changes one plan node's model-library assignment through the app service.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Plan, node, expected revision, and assignment.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns application validation, conflict, or persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_plan_node_assignment(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::PlanAssignmentRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        scriberig_app::SessionBackend::set_plan_node_assignment(&self.service, context, request)
    }

    application_plan_mutation_methods!();

    /// Reads safe expanded detail through the application-owned session service.
    ///
    /// - `context`: Caller authorization context.
    /// - `session`: Session containing the result.
    /// - `branch`: Branch containing the result.
    /// - `result`: Stable tool-result identity.
    ///
    /// Bounded allowlisted result detail.
    ///
    /// Returns the application authorization or lookup error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn tool_result_detail(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
        result: EntityId,
    ) -> Result<scriberig_protocol::ToolResultDetailResponse, ApplicationError> {
        self.service
            .tool_result_detail(context, session, branch, result)
            .map(|detail| scriberig_protocol::ToolResultDetailResponse {
                result: detail.result.to_string(),
                detail: detail.detail,
                truncated: detail.truncated,
            })
    }

    /// Projects upstream safe model snapshots into public protocol DTOs.
    ///
    /// - `context`: Caller authorization context.
    ///
    /// Safe upstream-derived model display rows.
    ///
    /// Returns the application model-library error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_library(
        &self,
        context: &ApplicationContext,
    ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
        let models = self.service.agent_model_library(context)?;
        Ok(model_library_response(models))
    }

    /// Updates future model selection through the shared application service.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Session and revision-checked profile selection.
    ///
    /// The authoritative updated execution selection.
    ///
    /// Returns the shared application validation or persistence error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_model(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::SessionModelSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        self.service.set_session_model(context, request)
    }

    /// Updates future thinking selection through the shared application service.
    ///
    /// - `context`: Caller authorization context.
    /// - `request`: Session and revision-checked thinking selection.
    ///
    /// The authoritative updated execution selection.
    ///
    /// Returns the shared application validation or persistence error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_thinking_level(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::SessionThinkingSelectionRequest,
    ) -> Result<scriberig_session::SessionExecutionSelection, ApplicationError> {
        self.service.set_session_thinking_level(context, request)
    }

    /// Submits an agent command through the public application boundary.
    ///
    /// - `context`: Caller authorization context.
    /// - `session`: Target session identity.
    /// - `branch`: Optional target branch.
    /// - `prompt`: User-authored command text.
    ///
    /// The accepted operation identity.
    ///
    /// Returns the application session error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
                anchor: None,
                prompt: prompt.to_owned(),
                selection: None,
            },
        )
    }

    application_handle_method!();
}

#[path = "service_application_query.rs"]
pub(super) mod query;
use query::model_library_response;
