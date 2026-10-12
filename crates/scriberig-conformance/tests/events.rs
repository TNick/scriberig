//! Verifies event references retain authoritative semantic handles.

use scriberig_client::Client;
use scriberig_conformance::fixture;
use scriberig_core::EntityId;
use scriberig_protocol::{
    EntityReference, Event, EventKind, OperationReference, SnapshotReference, SnapshotSelector,
};
use scriberig_server::InProcessTransport;

/// Verifies an event carries references rather than mutable semantic copies.
#[test]
fn event_references_survive_serialization() {
    let event = Event {
        sequence: 1,
        kind: EventKind::EntityChanged,
        entity: Some(EntityReference {
            id: EntityId::from_bytes([1; 16]).to_string(),
        }),
        revision: None,
        snapshot: Some(SnapshotReference {
            id: "snp_fixture".to_owned(),
        }),
        operation: Some(OperationReference {
            id: "op_fixture".to_owned(),
        }),
        metadata: vec![("authority".to_owned(), "canonical".to_owned())],
    };
    let encoded = serde_json::to_string(&event).expect("serialize event");
    let decoded: Event = serde_json::from_str(&encoded).expect("deserialize event");
    assert_eq!(decoded, event);
    assert_eq!(
        decoded.entity.expect("entity").id,
        EntityId::from_bytes([1; 16]).to_string()
    );
}

/// Verifies event handles resolve authoritative state after observation.
#[test]
fn event_subject_resolves_through_the_public_read_boundary() {
    let fixture = fixture();
    let client = Client::new(InProcessTransport::new(fixture.clone()));
    let delivered = client
        .subscribe(vec!["entity_changed".to_owned()])
        .expect("event subscription");
    assert_eq!(delivered.len(), 1);
    assert_eq!(
        delivered[0].entity.as_ref().expect("event entity").id,
        fixture.entity.to_string()
    );
    let event = Event {
        sequence: 2,
        kind: EventKind::EntityChanged,
        entity: Some(EntityReference {
            id: fixture.entity.to_string(),
        }),
        revision: None,
        snapshot: Some(SnapshotReference {
            id: fixture.snapshot.to_string(),
        }),
        operation: None,
        metadata: Vec::new(),
    };
    let response = client
        .inspect(
            event.entity.expect("event subject").id,
            SnapshotSelector::Exact(SnapshotReference {
                id: event.snapshot.expect("event snapshot").id,
            }),
            false,
        )
        .expect("authoritative inspection");
    assert_eq!(response.entity.id, fixture.entity.to_string());
    assert_eq!(response.revision.id, fixture.revision.to_string());
    assert_eq!(response.snapshot.id, fixture.snapshot.to_string());

    let reconnected = Client::new(InProcessTransport::new(fixture.clone()));
    let recovered = reconnected
        .subscribe(vec!["entity_changed".to_owned()])
        .expect("resubscribe after disconnect");
    assert_eq!(recovered, delivered);
    let recovered_inspection = reconnected
        .inspect(
            fixture.entity.to_string(),
            SnapshotSelector::Exact(SnapshotReference {
                id: fixture.snapshot.to_string(),
            }),
            false,
        )
        .expect("resolve recovered event");
    assert_eq!(
        recovered_inspection.revision.id,
        fixture.revision.to_string()
    );
}
