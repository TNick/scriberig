//! Executes universal application requests through an injected backend.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

#[path = "service_backend.rs"]
mod service_backend;
pub use service_backend::ApplicationBackend;
#[path = "service_clone.rs"]
mod service_clone;
#[path = "service_facade.rs"]
mod service_facade;
#[path = "service_query.rs"]
mod service_query;
use crate::{
    ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult, MigrationDetail,
    SessionBackend,
};
use scriberig_core::EntityId;
use scriberig_session::Session;

#[path = "service_sessions/plan_rollups.rs"]
mod plan_rollups;
#[path = "service_agent_console.rs"]
mod service_agent_console;
#[path = "service_agent_profile.rs"]
mod service_agent_profile;
#[path = "service_configuration.rs"]
mod service_configuration;
#[path = "service_execution.rs"]
mod service_execution;
#[path = "service_plans.rs"]
mod service_plans;
#[path = "service_projection_style.rs"]
mod service_projection_style;
#[path = "service_query_helpers.rs"]
mod service_query_helpers;
#[path = "service_render_style.rs"]
mod service_render_style;
#[path = "service_sessions.rs"]
mod service_sessions;
#[path = "session_failures.rs"]
mod session_failures;
#[path = "session_persistence.rs"]
mod session_persistence;

/// Facade that delegates every application operation to one injected backend.
///
/// Clones share the backend, operation registry, sessions, and agent identity
/// counter so host adapters can retain the same application boundary.
///
/// # Public members
/// None.
///
/// # Private members
/// - `backend`: Shared semantic backend.
/// - `operations`: Shared operation lifecycle registry.
/// - `sessions`: In-process session cache.
/// - `next_agent_handle`: Shared local agent identity counter.
/// - `session_store`: Optional durable session store.
/// - `agent_profile_provider`: Optional semantic agent-profile reader.
/// - `provider_extension_runtime`: Shared provider and extension generation.
/// - `extension_component_mailbox`: Optional upstream plugin UI registry.
/// - `presentation_preferences`: Agent presentation preference store.
/// - `projection_style`: Projection style configuration.
/// - `project_service`: Optional persisted project catalog for artifact inspection.
pub struct ApplicationService<B> {
    backend: Arc<B>,
    operations: crate::OperationRegistry,
    sessions: Arc<Mutex<BTreeMap<EntityId, Session>>>,
    next_agent_handle: Arc<AtomicU64>,
    session_store: Option<Arc<dyn scriberig_session::SessionStore>>,
    /// Optional shared semantic profile provider used by all invocation surfaces.
    agent_profile_provider: Option<Arc<dyn crate::AgentProfileProvider>>,
    pub(super) provider_extension_runtime: crate::ProviderExtensionRuntime,
    extension_component_mailbox: Option<rpi_extensions::ExtensionStatusMailbox>,
    presentation_preferences: Arc<dyn crate::AgentPresentationPreferenceStore>,
    projection_style: crate::ProjectionStyleConfiguration,
    pub(super) project_service: Option<Arc<scriberig_project::ProjectService>>,
    /// Optional external CAD providers configured by the product host.
    pub(super) cad_provider_registry: Arc<crate::cad_provider_registry::CadProviderRegistry>,
}

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Starts an agent operation at a session root through the shared boundary.
    ///
    /// - `self`: Application service owning the session subsystem.
    /// - `context`: Authorized application context.
    /// - `session`: Stable target session identity.
    /// - `prompt`: User-authored agent instruction.
    ///
    /// Stable operation identity for lifecycle observation.
    ///
    /// Returns the shared application failure from agent submission.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn start_agent(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        prompt: impl Into<String>,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        self.submit_agent_command(
            context,
            &crate::AgentCommandRequest {
                session,
                branch: None,
                anchor: None,
                prompt: prompt.into(),
                selection: None,
            },
        )
    }

    /// Continues an agent operation on an existing session branch.
    ///
    /// - `self`: Application service owning the session subsystem.
    /// - `context`: Authorized application context.
    /// - `session`: Stable target session identity.
    /// - `branch`: Stable branch receiving the follow-up.
    /// - `prompt`: User-authored follow-up instruction.
    ///
    /// Stable operation identity for lifecycle observation.
    ///
    /// Returns the shared application failure from agent submission.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn continue_agent(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
        prompt: impl Into<String>,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        self.submit_agent_command(
            context,
            &crate::AgentCommandRequest {
                session,
                branch: None,
                anchor: Some(scriberig_session::ContinuationAnchor::BranchHead { branch }),
                prompt: prompt.into(),
                selection: None,
            },
        )
    }

    /// Starts a new session and returns application allocated handles.
    ///
    /// - `self`: Application service owning session creation.
    /// - `context`: Authorized application context.
    /// - `prompt`: Initial user instruction.
    ///
    /// Operation, session, and root branch identities accepted by the service.
    ///
    /// Returns authorization, session persistence, or operation errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn start_new_session_agent(
        &self,
        context: &ApplicationContext,
        prompt: impl Into<String>,
    ) -> Result<crate::NewSessionAgentStart, ApplicationError>
    where
        B: ApplicationBackend,
    {
        self.start_new_session_agent_with_selection(
            context,
            crate::NewSessionAgentRequest {
                prompt: prompt.into(),
                selection: None,
            },
        )
    }

    /// Starts a new session with an optional exact semantic selection.
    ///
    /// - `self`: Application service owning session creation.
    /// - `context`: Authorized application context.
    /// - `request`: Initial instruction and optional semantic scope.
    ///
    /// Operation, session, and root branch identities accepted by the service.
    ///
    /// Returns authorization, semantic selection, persistence, or operation errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn start_new_session_agent_with_selection(
        &self,
        context: &ApplicationContext,
        request: crate::NewSessionAgentRequest,
    ) -> Result<crate::NewSessionAgentStart, ApplicationError>
    where
        B: ApplicationBackend,
    {
        let session = self.next_entity(0xA0);
        let operation = self.submit_agent_command(
            context,
            &crate::AgentCommandRequest {
                session,
                branch: None,
                anchor: None,
                prompt: request.prompt,
                selection: request.selection,
            },
        )?;
        let state = <Self as crate::SessionBackend>::read_session(
            self,
            context,
            &crate::SessionRequest { session },
        )?;
        let root_branch = state
            .branches
            .iter()
            .find(|branch| branch.parent.is_none())
            .map(|branch| branch.id)
            .ok_or_else(|| ApplicationError::Internal {
                message: "new session has no root branch".to_owned(),
            })?;
        Ok(crate::NewSessionAgentStart {
            operation,
            session,
            root_branch,
        })
    }

    /// Continues an agent operation from an exact historical session anchor.
    ///
    /// - `self`: Application service owning session continuation.
    /// - `context`: Authorized application context.
    /// - `session`: Stable session identity.
    /// - `anchor`: Exact validated continuation location.
    /// - `prompt`: User-authored follow-up instruction.
    ///
    /// Stable shared operation identity.
    ///
    /// Returns authorization, anchor validation, or backend operation errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn continue_agent_at(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        anchor: scriberig_session::ContinuationAnchor,
        prompt: impl Into<String>,
    ) -> Result<crate::AgentContinuationAccepted, ApplicationError>
    where
        B: ApplicationBackend,
    {
        self.continue_agent_at_request(
            context,
            crate::AgentContinueAtRequest {
                session,
                anchor,
                prompt: prompt.into(),
                selection: None,
            },
        )
    }

    /// Continues at an exact anchor with optional semantic scope.
    ///
    /// - `self`: Application service owning session continuation.
    /// - `context`: Authorized application context.
    /// - `request`: Session, exact anchor, prompt, and semantic selection.
    ///
    /// Stable operation, session, and child branch identities.
    ///
    /// Returns authorization, anchor validation, or backend operation errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn continue_agent_at_request(
        &self,
        context: &ApplicationContext,
        request: crate::AgentContinueAtRequest,
    ) -> Result<crate::AgentContinuationAccepted, ApplicationError>
    where
        B: ApplicationBackend,
    {
        let session = request.session;
        let anchor = request.anchor;
        let prompt = request.prompt;
        let operation = self.submit_agent_command(
            context,
            &crate::AgentCommandRequest {
                session,
                branch: None,
                anchor: Some(anchor),
                prompt: prompt.clone(),
                selection: request.selection,
            },
        )?;
        let state = <Self as crate::SessionBackend>::read_session(
            self,
            context,
            &crate::SessionRequest { session },
        )?;
        let branch = state
            .branches
            .iter()
            .find(|branch| {
                branch
                    .continuation
                    .is_some_and(|origin| origin.anchor == anchor)
                    && branch.turns.iter().any(|turn| {
                        turn.human
                            .as_ref()
                            .is_some_and(|input| input.text == prompt)
                    })
            })
            .map(|branch| branch.id)
            .ok_or_else(|| ApplicationError::Internal {
                message: "accepted continuation has no durable branch".to_owned(),
            })?;
        Ok(crate::AgentContinuationAccepted {
            operation,
            session,
            branch,
        })
    }

    /// Reads one migration proposal through the shared application boundary.
    ///
    /// - `self`: Application service.
    /// - `context`: Authorized application context.
    /// - `id`: Stable migration handle.
    ///
    /// Snapshot-bound migration detail.
    ///
    /// Returns the backend's structured application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn migration_detail(
        &self,
        context: &ApplicationContext,
        id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        self.backend.migration_detail(context, id)
    }
}
