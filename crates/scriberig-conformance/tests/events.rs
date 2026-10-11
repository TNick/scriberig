//! Verifies event references retain authoritative semantic handles.

use scriberig_core::EntityId;
use scriberig_protocol::{
    EntityReference, Event, EventKind, OperationReference, SnapshotReference,
};

/// Verifies an event carries references rather than mutable semantic copies.
#[test]
fn event_references_survive_serialization() {
    let event = Event {
        kind: EventKind::EntityChanged,
        entity: Some(EntityReference {
            id: EntityId::new([1; 16]).to_string(),
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
        EntityId::new([1; 16]).to_string()
    );
}
