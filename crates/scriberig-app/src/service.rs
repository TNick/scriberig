//! Executes universal application requests through an injected backend.

use crate::{
    ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult, DetailLevel,
    InspectRequest, InspectResult, QueryService,
};
use scriberig_query::{DetailLevel as QueryDetailLevel, Inspection, TargetRef};

/// Provides semantic execution for the universal application request model.
pub trait ApplicationBackend {
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

impl<B: ApplicationBackend> crate::SessionBackend for ApplicationService<B> {
    /// Delegates a session read to the application backend.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Session request to resolve.
    ///
    /// # Returns
    /// The requested session.
    ///
    /// # Errors
    /// Returns the backend session error.
    fn read_session(
        &self,
        context: &ApplicationContext,
        request: &crate::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        self.backend.read_session(context, request)
    }

    /// Delegates agent command submission to the application backend.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Agent command request to submit.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns the backend agent error.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        request: &crate::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        self.backend.submit_agent_command(context, request)
    }
}

/// Converts the universal query inspection into the application result shape.
///
/// # Parameters
/// - `inspection`: Query-layer inspection result.
/// - `_detail`: Detail requested by the application caller.
///
/// # Returns
/// Snapshot-aware application inspection metadata.
///
/// # Errors
/// Returns a revision mismatch when the query target is inconsistent.
fn inspect_result(
    inspection: Inspection,
    _detail: DetailLevel,
) -> Result<InspectResult, ApplicationError> {
    let entity = match inspection.subject {
        TargetRef::Snapshot(target) => target.entity,
        TargetRef::Revision(target) => target.entity,
    };
    let revision = match inspection.subject {
        TargetRef::Revision(target) => {
            if inspection
                .revision
                .is_some_and(|value| value != target.revision)
            {
                return Err(ApplicationError::RevisionMismatch {
                    entity,
                    revision: target.revision,
                });
            }
            Some(target.revision)
        }
        TargetRef::Snapshot(_) => inspection.revision,
    };
    Ok(InspectResult {
        entity,
        revision,
        snapshot: inspection.snapshot,
        object_type: inspection.entity_type,
        label: inspection.label,
        description: inspection.description,
        capabilities: inspection
            .operations
            .into_iter()
            .map(|capability| capability.id)
            .chain(inspection.views.into_iter().map(|capability| capability.id))
            .collect(),
        trust: None,
    })
}

/// Converts an application inspection request into an exact query target.
///
/// # Parameters
/// - `request`: Application inspection request.
/// - `current`: Resolved current snapshot when the request uses `Current`.
///
/// # Returns
/// Query-layer inspection request with an exact target.
///
/// # Errors
/// Returns an unsupported-operation error when no exact snapshot or revision
/// was supplied.
fn query_request(
    request: &InspectRequest,
    current: Option<scriberig_core::SnapshotId>,
) -> Result<scriberig_query::InspectRequest, ApplicationError> {
    let target = match request.snapshot {
        crate::SnapshotResolution::At(snapshot) => {
            TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
                entity: request.entity,
                snapshot,
            })
        }
        crate::SnapshotResolution::Revision(revision) => {
            TargetRef::Revision(scriberig_query::EntityAtRevision {
                entity: request.entity,
                revision,
            })
        }
        crate::SnapshotResolution::Current => {
            TargetRef::Snapshot(scriberig_query::EntityInSnapshot {
                entity: request.entity,
                snapshot: current.ok_or_else(|| ApplicationError::Internal {
                    message: "current snapshot was not resolved".to_owned(),
                })?,
            })
        }
    };
    Ok(scriberig_query::InspectRequest {
        target,
        detail: match request.detail {
            DetailLevel::Compact => QueryDetailLevel::Compact,
            DetailLevel::Detailed => QueryDetailLevel::Structure,
        },
        view: None,
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
