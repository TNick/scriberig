//! Executes universal application requests through an injected backend.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use crate::{
    ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult, InspectRequest,
    InspectResult, MigrationDetail, QueryService,
};
use scriberig_core::EntityId;
use scriberig_session::Session;

use crate::SessionBackend;

use self::service_query_helpers::{inspect_result, query_request};

#[path = "service_agent_console.rs"]
mod service_agent_console;
#[path = "service_configuration.rs"]
mod service_configuration;
#[path = "service_query_helpers.rs"]
mod service_query_helpers;
#[path = "service_sessions.rs"]
mod service_sessions;
#[path = "session_failures.rs"]
mod session_failures;
#[path = "session_persistence.rs"]
mod session_persistence;

/// Provides semantic execution for the universal application request model.
pub trait ApplicationBackend {
    /// Reads canonical detail for one migration proposal.
    ///
    /// # Parameters
    /// - `_context`: Authorized application context.
    /// - `_id`: Stable migration handle.
    ///
    /// # Returns
    /// Snapshot-bound migration detail.
    ///
    /// # Errors
    /// Returns unsupported-operation when proposal reads are not configured.
    fn migration_detail(
        &self,
        _context: &ApplicationContext,
        _id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "migration detail".to_owned(),
        })
    }

    /// Executes one transport-independent application request.
    ///
    /// # Parameters
    /// - `self`: Backend owning semantic subsystem providers.
    /// - `context`: Caller, capability, and request context.
    /// - `request`: Universal application request to execute.
    ///
    /// # Returns
    /// Structured application result owned by the canonical subsystem.
    ///
    /// # Errors
    /// Returns a structured application failure without transport details.
    fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError>;

    /// Resolves the current accepted snapshot for the request.
    ///
    /// # Parameters
    /// - `self`: Backend owning snapshot/root state.
    /// - `_context`: Caller context.
    ///
    /// # Returns
    /// The exact current snapshot.
    ///
    /// # Errors
    /// Returns an unsupported-operation error when no current root exists.
    fn current_snapshot(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_core::SnapshotId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "current snapshot resolution".to_owned(),
        })
    }

    /// Reads a session through the owning application subsystem.
    ///
    /// # Parameters
    /// - `_context`: Application context for the request.
    /// - `_request`: Session request to resolve.
    ///
    /// # Returns
    /// The requested session.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _request: &crate::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session read".to_owned(),
        })
    }

    /// Atomically persists a session execution selection through its owner.
    ///
    /// # Parameters
    /// - `_context`: Authorized application context.
    /// - `_session`: Session snapshot with the updated selection.
    /// - `_expected_revision`: Selection revision observed by the caller; the
    ///   owner must compare it atomically with the persisted revision.
    ///
    /// # Returns
    /// Unit after the update is persisted.
    ///
    /// # Errors
    /// Returns conflict for a stale revision, or unsupported-operation when
    /// the backend cannot atomically mutate session state.
    fn update_session_execution_selection(
        &self,
        _context: &ApplicationContext,
        _session: &scriberig_session::Session,
        _expected_revision: u64,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session execution selection update".to_owned(),
        })
    }

    /// Starts an agent command through the owning application subsystem.
    ///
    /// # Parameters
    /// - `_context`: Application context for the request.
    /// - `_request`: Agent command request to submit.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns an unsupported-operation error by default.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _request: &crate::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "agent command submission".to_owned(),
        })
    }
}

/// Facade that delegates every application operation to one injected backend.
pub struct ApplicationService<B> {
    backend: B,
    operations: crate::OperationRegistry,
    sessions: Arc<Mutex<BTreeMap<EntityId, Session>>>,
    next_agent_handle: AtomicU64,
    session_store: Option<Arc<dyn scriberig_session::SessionStore>>,
    model_library: crate::ModelLibrary,
    presentation_preferences: Arc<dyn crate::AgentPresentationPreferenceStore>,
}

impl<B> ApplicationService<B> {
    /// Creates an application service around canonical subsystem providers.
    ///
    /// # Parameters
    /// - `backend`: Backend implementing universal operation execution.
    ///
    /// # Returns
    /// A transport-independent application service.
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            operations: crate::OperationRegistry::new(),
            sessions: Arc::new(Mutex::new(BTreeMap::new())),
            next_agent_handle: AtomicU64::new(1),
            session_store: None,
            model_library: crate::ModelLibrary::default(),
            presentation_preferences: Arc::new(
                crate::InMemoryAgentPresentationPreferenceStore::default(),
            ),
        }
    }

    /// Returns the injected backend by shared reference.
    ///
    /// # Parameters
    /// - `self`: Application service to inspect.
    ///
    /// # Returns
    /// The backend used for semantic execution.
    #[must_use]
    pub const fn backend(&self) -> &B {
        &self.backend
    }

    /// Returns the lifecycle registry used by this application service.
    #[must_use]
    pub const fn operations(&self) -> &crate::OperationRegistry {
        &self.operations
    }
}

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Starts an agent operation at a session root through the shared boundary.
    ///
    /// # Parameters
    /// - `self`: Application service owning the session subsystem.
    /// - `context`: Authorized application context.
    /// - `session`: Stable target session identity.
    /// - `prompt`: User-authored agent instruction.
    ///
    /// # Returns
    /// Stable operation identity for lifecycle observation.
    ///
    /// # Errors
    /// Returns the shared application failure from agent submission.
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
                prompt: prompt.into(),
                selection: None,
            },
        )
    }

    /// Continues an agent operation on an existing session branch.
    ///
    /// # Parameters
    /// - `self`: Application service owning the session subsystem.
    /// - `context`: Authorized application context.
    /// - `session`: Stable target session identity.
    /// - `branch`: Stable branch receiving the follow-up.
    /// - `prompt`: User-authored follow-up instruction.
    ///
    /// # Returns
    /// Stable operation identity for lifecycle observation.
    ///
    /// # Errors
    /// Returns the shared application failure from agent submission.
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
                branch: Some(branch),
                prompt: prompt.into(),
                selection: None,
            },
        )
    }

    /// Reads one migration proposal through the shared application boundary.
    ///
    /// # Parameters
    /// - `self`: Application service.
    /// - `context`: Authorized application context.
    /// - `id`: Stable migration handle.
    ///
    /// # Returns
    /// Snapshot-bound migration detail.
    ///
    /// # Errors
    /// Returns the backend's structured application error.
    pub fn migration_detail(
        &self,
        context: &ApplicationContext,
        id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        self.backend.migration_detail(context, id)
    }

    /// Executes one universal application request.
    ///
    /// # Parameters
    /// - `self`: Application service.
    /// - `context`: Caller and capability context.
    /// - `request`: Universal request to execute.
    ///
    /// # Returns
    /// The structured result returned by the owning subsystem.
    ///
    /// # Errors
    /// Returns a structured application failure.
    pub fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        self.backend.execute(context, request)
    }
}

impl<B: ApplicationBackend> QueryService for ApplicationService<B> {
    /// Executes an application inspection request through the universal path.
    ///
    /// # Parameters
    /// - `self`: Application service.
    /// - `context`: Caller and capability context.
    /// - `request`: Snapshot-aware application inspection request.
    ///
    /// # Returns
    /// Application inspection metadata in the CLI/server-facing shape.
    ///
    /// # Errors
    /// Returns a structured application failure when the operation fails or
    /// returns a non-inspection result.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        let current = match request.snapshot {
            crate::SnapshotResolution::Current => Some(self.backend.current_snapshot(context)?),
            _ => None,
        };
        let application_request = ApplicationRequest::Inspect(query_request(request, current)?);
        let result = self.execute(context, &application_request)?;
        let ApplicationResult::Inspect(inspection) = result else {
            return Err(ApplicationError::Internal {
                message: "inspect backend returned a different result kind".to_owned(),
            });
        };
        inspect_result(inspection, request.detail)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
