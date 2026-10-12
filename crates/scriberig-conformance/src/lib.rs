//! Defines deterministic fixtures shared by cross-surface conformance tests.

pub mod agent_fixture;

use scriberig_agent_core::{INSPECT_CAPABILITY, InspectTool};
use scriberig_app::{
    ApplicationContext, ApplicationError, CapabilityContext, CapabilityId, InspectRequest,
    InspectResult, QueryService, RuntimeCapability, SnapshotResolution,
};
use scriberig_core::{EncodingVersion, EntityId, ObjectType, RevisionId, SnapshotId};
use scriberig_server::ApplicationService;
use scriberig_store::{Revision, RevisionStore, SnapshotChange, SnapshotStore};
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
    /// In-memory storage backend containing the fixture state.
    pub store: std::sync::Arc<MemoryStore>,
}

/// Creates one deterministic fixture with no external services.
///
/// # Returns
/// A fixture containing one known entity, revision, and snapshot.
///
/// # Panics
/// Panics only if the fixed fixture type or storage records cannot be created.
#[must_use]
pub fn fixture() -> Fixture {
    let store = std::sync::Arc::new(MemoryStore::new());
    let entity = EntityId::from_bytes([1; 16]);
    let object_type = ObjectType::new("markdown.document").expect("static type");
    let revision = store
        .put(
            Revision::new(EncodingVersion::V1, object_type, b"architecture".to_vec())
                .expect("fixture revision"),
        )
        .expect("store fixture revision");
    let snapshot = store
        .apply_changes(
            store.empty_snapshot(),
            &[SnapshotChange::Set(entity, revision)],
        )
        .expect("store fixture snapshot");
    Fixture {
        entity,
        revision,
        snapshot,
        store,
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
        SnapshotResolution::Current => fixture.snapshot,
        SnapshotResolution::At(snapshot) if snapshot == fixture.snapshot => snapshot,
        SnapshotResolution::At(snapshot) => {
            return Err(ApplicationError::SnapshotUnavailable { snapshot });
        }
        SnapshotResolution::Revision(revision) if revision == fixture.revision => fixture.snapshot,
        SnapshotResolution::Revision(revision) => {
            return Err(ApplicationError::RevisionMismatch {
                entity: fixture.entity,
                revision,
            });
        }
    };
    let resolved = fixture
        .store
        .resolve(snapshot, fixture.entity)
        .map_err(|error| ApplicationError::Internal {
            message: format!("fixture resolution failed: {error:?}"),
        })?;
    if resolved != Some(fixture.revision) {
        return Err(ApplicationError::Internal {
            message: "fixture snapshot resolved an unexpected revision".to_owned(),
        });
    }
    Ok(InspectResult {
        entity: fixture.entity,
        revision: Some(fixture.revision),
        snapshot: Some(snapshot),
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

    /// Returns a deterministic operation lifecycle observation.
    ///
    /// # Parameters
    /// - `operation_id`: Stable operation handle.
    ///
    /// # Returns
    /// Protocol operation status selected by the stable test handle.
    ///
    /// # Errors
    /// This fixture never fails operation lookup.
    fn operation_status(
        &self,
        operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        let (status, percent, message, error) = match operation_id {
            "op-queued" => (
                scriberig_protocol::operation::OperationStatus::Queued,
                0,
                "queued",
                None,
            ),
            "op-running" => (
                scriberig_protocol::operation::OperationStatus::Running,
                50,
                "running",
                None,
            ),
            "op-failed" => (
                scriberig_protocol::operation::OperationStatus::Failed,
                50,
                "failed",
                Some("deterministic failure"),
            ),
            "op-cancelled" => (
                scriberig_protocol::operation::OperationStatus::Cancelled,
                25,
                "cancelled",
                Some("cancelled by caller"),
            ),
            _ => (
                scriberig_protocol::operation::OperationStatus::Completed,
                100,
                "complete",
                None,
            ),
        };
        let result_snapshot = matches!(
            &status,
            scriberig_protocol::operation::OperationStatus::Completed
        )
        .then(|| scriberig_protocol::SnapshotReference {
            id: self.snapshot.to_string(),
        });
        Ok(scriberig_protocol::Operation {
            reference: scriberig_protocol::OperationReference {
                id: operation_id.to_owned(),
            },
            kind: scriberig_protocol::operation::OperationKind::Inspect,
            status,
            progress: Some(scriberig_protocol::OperationProgress {
                percent,
                message: Some(message.to_owned()),
            }),
            result_snapshot,
            result_handles: Vec::new(),
            agent_result: None,
            error: error.map(str::to_owned),
        })
    }

    /// Returns the deterministic cancelled state for one operation.
    ///
    /// # Parameters
    /// - `operation_id`: Stable operation handle.
    ///
    /// # Returns
    /// Cancelled protocol operation.
    ///
    /// # Errors
    /// This fixture never fails cancellation lookup.
    fn cancel_operation(
        &self,
        operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        let mut operation = self.operation_status(operation_id)?;
        operation.status = scriberig_protocol::operation::OperationStatus::Cancelled;
        operation.result_snapshot = None;
        operation.error = Some("cancelled by caller".to_owned());
        Ok(operation)
    }

    /// Returns one authoritative event for the deterministic fixture.
    ///
    /// # Parameters
    /// - `event_kinds`: Optional protocol event-name filter.
    ///
    /// # Returns
    /// A stable entity-change event when selected by the filter.
    ///
    /// # Errors
    /// This fixture does not fail event reads.
    fn events(
        &self,
        event_kinds: &[String],
    ) -> Result<Vec<scriberig_protocol::Event>, ApplicationError> {
        let selected = event_kinds.is_empty()
            || event_kinds
                .iter()
                .any(|kind| kind == "EntityChanged" || kind.eq_ignore_ascii_case("entity_changed"));
        if !selected {
            return Ok(Vec::new());
        }
        Ok(vec![scriberig_protocol::Event {
            sequence: 1,
            kind: scriberig_protocol::EventKind::EntityChanged,
            entity: Some(scriberig_protocol::EntityReference {
                id: self.entity.to_string(),
            }),
            revision: Some(scriberig_protocol::RevisionReference {
                entity: scriberig_protocol::EntityReference {
                    id: self.entity.to_string(),
                },
                id: self.revision.to_string(),
            }),
            snapshot: Some(scriberig_protocol::SnapshotReference {
                id: self.snapshot.to_string(),
            }),
            operation: None,
            metadata: vec![("authority".to_owned(), "canonical".to_owned())],
        }])
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
