//! Tests the application service facade.

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
        let entity = EntityId::from_bytes([1; 16]);
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

    /// Resolves the fixture's exact current snapshot.
    ///
    /// # Parameters
    /// - `self`: Fixture backend.
    /// - `_context`: Unused test context.
    ///
    /// # Returns
    /// The deterministic current snapshot.
    ///
    /// # Errors
    /// Returns an error when the operation cannot complete.
    fn current_snapshot(
        &self,
        _context: &ApplicationContext,
    ) -> Result<SnapshotId, ApplicationError> {
        Ok(SnapshotId::new([3; 32]))
    }
}

/// Verifies the typed inspection facade uses the universal execution path.
#[test]
fn inspection_delegates_through_application_backend() {
    let service = ApplicationService::new(Fixture);
    let request = InspectRequest {
        entity: EntityId::from_bytes([1; 16]),
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

/// Verifies omitted current state is resolved before universal execution.
#[test]
fn current_inspection_is_resolved_to_one_exact_snapshot() {
    let service = ApplicationService::new(Fixture);
    let request = InspectRequest {
        entity: EntityId::from_bytes([1; 16]),
        snapshot: SnapshotResolution::Current,
        detail: DetailLevel::Compact,
    };
    let result = service
        .inspect(&ApplicationContext::empty(), &request)
        .expect("current inspection");
    assert_eq!(result.snapshot, Some(SnapshotId::new([3; 32])));
}
