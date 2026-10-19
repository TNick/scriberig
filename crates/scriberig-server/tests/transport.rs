//! Verifies semantic inspect behavior across the in-process server/client boundary.

use scriberig_app::{
    ApplicationContext, ApplicationError, CapabilityId, InspectRequest, InspectResult,
    RuntimeCapability,
};
use scriberig_client::{Client, ClientError};
use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};
use scriberig_protocol::{
    EntityReference, InspectEntityRequest, ProtocolTransport, ProtocolVersion, Request,
    RequestEnvelope, Response, SnapshotReference, SnapshotSelector,
};
use scriberig_server::{ApplicationService, InProcessTransport};

/// Deterministic application fixture used by server/client tests.
struct Fixture {
    entity: EntityId,
    revision: RevisionId,
    snapshot: SnapshotId,
}

impl ApplicationService for Fixture {
    /// Executes fixture inspection semantics.
    ///
    /// - `_context`: Request context.
    /// - `request`: Inspection request.
    ///
    /// Fixture inspection result.
    ///
    /// Returns `NotFound` for unknown entities.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        if request.entity != self.entity {
            return Err(ApplicationError::NotFound {
                subject: request.entity.to_string(),
            });
        }
        Ok(InspectResult {
            entity: self.entity,
            revision: Some(self.revision),
            snapshot: Some(self.snapshot),
            object_type: Some(ObjectType::new("markdown.document").expect("type")),
            label: Some("Architecture".to_owned()),
            description: Some("Fixture document".to_owned()),
            capabilities: vec!["inspect".to_owned()],
            trust: None,
        })
    }

    /// Returns fixture capability metadata.
    ///
    /// One inspect capability.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        vec![RuntimeCapability::new(
            CapabilityId::new("inspect_entity").expect("capability"),
            "Inspect one entity",
            true,
        )]
    }

    /// Returns a completed fixture operation.
    ///
    /// - `operation_id`: Operation identity.
    ///
    /// Completed operation status.
    ///
    /// This fixture does not fail operation lookup.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
            progress: None,
            result_snapshot: Some(SnapshotReference {
                id: self.snapshot.to_string(),
            }),
            result_handles: Vec::new(),
            agent_result: None,
            error: None,
        })
    }
}

/// Creates a deterministic fixture.
///
/// Fixture application service.
/// # Returns
/// The operation result.
fn fixture() -> Fixture {
    Fixture {
        entity: EntityId::from_bytes([1; 16]),
        revision: RevisionId::new([2; 32]),
        snapshot: SnapshotId::new([3; 32]),
    }
}

/// Verifies inspect preserves exact entity, revision, and snapshot handles.
#[test]
fn inspect_round_trip_matches_direct_application_semantics() {
    let fixture = fixture();
    let expected = fixture
        .inspect(
            &ApplicationContext::empty(),
            &InspectRequest {
                entity: fixture.entity,
                snapshot: scriberig_app::SnapshotResolution::At(fixture.snapshot),
                detail: scriberig_app::DetailLevel::Compact,
            },
        )
        .expect("direct inspect");
    let transport = InProcessTransport::new(fixture);
    let client = Client::new(transport);
    let actual = client
        .inspect(
            expected.entity.to_string(),
            SnapshotSelector::Exact(SnapshotReference {
                id: expected.snapshot.expect("snapshot").to_string(),
            }),
            false,
        )
        .expect("remote inspect");
    assert_eq!(actual.entity.id, expected.entity.to_string());
    assert_eq!(
        actual.revision.id,
        expected.revision.expect("revision").to_string()
    );
    assert_eq!(
        actual.snapshot.id,
        expected.snapshot.expect("snapshot").to_string()
    );
}

/// Verifies missing entities retain the structured `NotFound` category.
#[test]
fn missing_entity_is_structured() {
    let client = Client::new(InProcessTransport::new(fixture()));
    let error = client
        .inspect(
            EntityId::from_bytes([7; 16]).to_string(),
            SnapshotSelector::Current,
            false,
        )
        .expect_err("missing entity");
    assert!(matches!(
        error,
        ClientError::Protocol(error)
            if error.code == scriberig_protocol::ProtocolErrorCode::NotFound
    ));
}

/// Verifies runtime capability discovery crosses the same transport.
#[test]
fn capabilities_round_trip() {
    let client = Client::new(InProcessTransport::new(fixture()));
    let response = client.capabilities().expect("capabilities");
    assert_eq!(response.capabilities[0].id, "inspect_entity");
}

/// Verifies unsupported protocol majors are rejected before application dispatch.
#[test]
fn unsupported_protocol_major_is_rejected() {
    let transport = InProcessTransport::new(fixture());
    let response = transport.send(RequestEnvelope {
        version: ProtocolVersion {
            major: 99,
            minor: 0,
        },
        correlation_id: "test-1".to_owned(),
        request: Request::Inspect(InspectEntityRequest {
            entity: EntityReference {
                id: EntityId::from_bytes([1; 16]).to_string(),
            },
            snapshot: SnapshotSelector::Current,
            detail: scriberig_protocol::request::DetailLevel::Compact,
        }),
    });
    assert!(
        matches!(response.response, Response::Error(error) if error.code == scriberig_protocol::ProtocolErrorCode::UnsupportedVersion)
    );
}
