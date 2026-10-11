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
}

/// Facade that delegates every application operation to one injected backend.
pub struct ApplicationService<B> {
    backend: B,
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
    pub const fn new(backend: B) -> Self {
        Self { backend }
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
mod tests {
    use super::{ApplicationBackend, ApplicationService};
    use crate::{
        ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult, DetailLevel,
        InspectRequest, QueryService, SnapshotResolution,
    };
    use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};
    use scriberig_query::{EntityAtRevision, Inspection, TargetRef};

    /// Provides one deterministic backend inspection for service tests.
    struct Fixture;

    impl ApplicationBackend for Fixture {
        /// Executes the fixture inspection request.
        ///
        /// # Parameters
        /// - `self`: Fixture backend.
        /// - `_context`: Unused test context.
        /// - `request`: Request expected to be an inspection.
        ///
        /// # Returns
        /// One deterministic inspection result.
        ///
        /// # Errors
        /// Returns an internal error for another request kind.
        fn execute(
            &self,
            _context: &ApplicationContext,
            request: &ApplicationRequest,
        ) -> Result<ApplicationResult, ApplicationError> {
            let ApplicationRequest::Inspect(_) = request else {
                return Err(ApplicationError::UnsupportedOperation {
                    operation: "fixture".to_owned(),
                });
            };
            let entity = EntityId::new([1; 16]);
            let revision = RevisionId::new([2; 32]);
            let snapshot = SnapshotId::new([3; 32]);
            Ok(ApplicationResult::Inspect(Inspection {
                subject: TargetRef::Revision(EntityAtRevision { entity, revision }),
                snapshot: Some(snapshot),
                revision: Some(revision),
                entity_type: Some(ObjectType::new("test.entity").expect("valid type")),
                label: Some("fixture".to_owned()),
                description: Some("test inspection".to_owned()),
                properties: Vec::new(),
                counts: Vec::new(),
                relations: Vec::new(),
                collections: Vec::new(),
                views: Vec::new(),
                operations: Vec::new(),
                provenance: Vec::new(),
                information_cost: None,
                detail: scriberig_query::DetailLevel::Identity,
            }))
        }
    }

    /// Verifies the typed inspection facade uses the universal execution path.
    #[test]
    fn inspection_delegates_through_application_backend() {
        let service = ApplicationService::new(Fixture);
        let request = InspectRequest {
            entity: EntityId::new([1; 16]),
            snapshot: SnapshotResolution::At(SnapshotId::new([3; 32])),
            detail: DetailLevel::Compact,
        };
        let result = service
            .inspect(&ApplicationContext::empty(), &request)
            .expect("inspection");
        assert_eq!(result.entity, request.entity);
        assert_eq!(result.revision, Some(RevisionId::new([2; 32])));
        assert_eq!(result.snapshot, Some(SnapshotId::new([3; 32])));
    }
}
