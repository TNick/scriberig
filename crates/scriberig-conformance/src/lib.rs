//! Defines deterministic fixtures shared by cross-surface conformance tests.

use scriberig_agent_core::{INSPECT_CAPABILITY, InspectTool};
use scriberig_app::{
    ApplicationContext, ApplicationError, CapabilityContext, CapabilityId, InspectRequest,
    InspectResult, QueryService, RuntimeCapability, SnapshotResolution,
};
use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};
use scriberig_server::ApplicationService;
use scriberig_store_memory::MemoryStore;

/// Deterministic semantic fixture used by every public-surface test.
#[derive(Clone)]
pub struct Fixture {
    /// Known inspectable entity identity.
    pub entity: EntityId,
    /// Known immutable entity revision.
    pub revision: RevisionId,
    /// Known immutable semantic snapshot.
    pub snapshot: SnapshotId,
    /// In-memory storage backend reserved for fixture state.
    pub store: std::sync::Arc<MemoryStore>,
}

/// Creates one deterministic fixture with no external services.
///
/// # Returns
/// A fixture containing one known entity, revision, and snapshot.
#[must_use]
pub fn fixture() -> Fixture {
    Fixture {
        entity: EntityId::new([1; 16]),
        revision: RevisionId::new([2; 32]),
        snapshot: SnapshotId::new([3; 32]),
        store: std::sync::Arc::new(MemoryStore::new()),
    }
}

/// Creates an application context granting the inspect capability.
///
/// # Returns
/// Deterministic trusted context.
///
/// # Panics
/// Panics only if the static capability identifier is invalid.
#[must_use]
pub fn allowed_context() -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new(INSPECT_CAPABILITY).expect("static capability")
        ]),
        ..ApplicationContext::empty()
    }
}

/// Builds the shared inspection result for one fixture request.
///
/// # Parameters
/// - `fixture`: Deterministic semantic fixture.
/// - `request`: Snapshot-aware inspection request.
///
/// # Returns
/// Fixture inspection result.
///
/// # Errors
/// Returns `NotFound` for an unknown entity.
fn inspect_result(
    fixture: &Fixture,
    request: &InspectRequest,
) -> Result<InspectResult, ApplicationError> {
    if request.entity != fixture.entity {
        return Err(ApplicationError::NotFound {
            subject: request.entity.to_string(),
        });
    }
    let snapshot = match request.snapshot {
        SnapshotResolution::Current
        | SnapshotResolution::At(_)
        | SnapshotResolution::Revision(_) => Some(fixture.snapshot),
    };
    Ok(InspectResult {
        entity: fixture.entity,
        revision: Some(fixture.revision),
        snapshot,
        object_type: Some(ObjectType::new("markdown.document").expect("static type")),
        label: Some("Architecture".to_owned()),
        description: Some("Deterministic conformance document".to_owned()),
        capabilities: vec!["inspect.entity".to_owned(), "handle:entity".to_owned()],
        trust: None,
    })
}

impl QueryService for Fixture {
    /// Executes the deterministic fixture query.
    ///
    /// # Parameters
    /// - `self`: Fixture application service.
    /// - `_context`: Trusted application context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// # Returns
    /// Fixture inspection result.
    ///
    /// # Errors
    /// Returns `NotFound` for unknown entities.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        inspect_result(self, request)
    }
}

impl ApplicationService for Fixture {
    /// Executes the same deterministic fixture query through the server boundary.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// # Returns
    /// Fixture inspection result.
    ///
    /// # Errors
    /// Returns `NotFound` for unknown entities.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        inspect_result(self, request)
    }

    /// Returns the fixture's inspect capability.
    ///
    /// # Returns
    /// One available runtime capability.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        vec![RuntimeCapability::new(
            CapabilityId::new("inspect_entity").expect("static capability"),
            "Inspect one entity",
            true,
        )]
    }

    /// Returns a deterministic completed operation.
    ///
    /// # Parameters
    /// - `operation_id`: Stable operation handle.
    ///
    /// # Returns
    /// Completed protocol operation.
    ///
    /// # Errors
    /// This fixture never fails operation lookup.
    fn operation_status(
        &self,
        operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        Ok(scriberig_protocol::Operation {
            reference: scriberig_protocol::OperationReference {
                id: operation_id.to_owned(),
            },
            kind: scriberig_protocol::operation::OperationKind::Inspect,
            status: scriberig_protocol::operation::OperationStatus::Completed,
            progress: Some(scriberig_protocol::OperationProgress {
                percent: 100,
                message: Some("complete".to_owned()),
            }),
            result_snapshot: Some(scriberig_protocol::SnapshotReference {
                id: self.snapshot.to_string(),
            }),
            error: None,
        })
    }
}

/// Constructs the provider-neutral inspect adapter for a fixture.
///
/// # Returns
/// An inspect tool backed by the shared application query implementation.
#[must_use]
pub fn inspect_tool() -> InspectTool<Fixture> {
    InspectTool::new(fixture())
}
