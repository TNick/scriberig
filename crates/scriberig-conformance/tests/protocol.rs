//! Verifies public protocol values round trip without relying on Debug output.

use scriberig_protocol::{
    EntityReference, Event, EventKind, InspectEntityRequest, Operation, OperationProgress,
    OperationReference, ProtocolError, ProtocolErrorCode, ProtocolVersion, Request,
    RequestEnvelope, SnapshotReference, SnapshotSelector,
};

/// Serializes and deserializes one protocol value.
fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let encoded = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&encoded).expect("deserialize")
}

/// Verifies inspect request envelopes retain correlation and snapshot context.
#[test]
fn inspect_request_round_trip() {
    let value = RequestEnvelope {
        version: ProtocolVersion::default(),
        correlation_id: "corr-1".to_owned(),
        request: Request::Inspect(InspectEntityRequest {
            entity: EntityReference {
                id: "ent_0101".to_owned(),
            },
            snapshot: SnapshotSelector::Exact(SnapshotReference {
                id: "snp_0202".to_owned(),
            }),
            detail: scriberig_protocol::request::DetailLevel::Deep,
        }),
    };
    assert_eq!(round_trip(&value), value);
}

/// Verifies structured protocol errors retain their machine-readable category.
#[test]
fn error_round_trip() {
    let value = ProtocolError {
        code: ProtocolErrorCode::PermissionDenied,
        message: "denied".to_owned(),
        details: Some(serde_json::json!({"scope": "entity"})),
        correlation_id: Some("corr-2".to_owned()),
    };
    assert_eq!(round_trip(&value), value);
}

/// Verifies operation status and event references retain stable handles.
#[test]
fn operation_and_event_round_trip() {
    let operation = Operation {
        reference: OperationReference {
            id: "op-1".to_owned(),
        },
        kind: scriberig_protocol::operation::OperationKind::Inspect,
        status: scriberig_protocol::operation::OperationStatus::Completed,
        progress: Some(OperationProgress {
            percent: 100,
            message: Some("done".to_owned()),
        }),
        result_snapshot: None,
        error: None,
    };
    assert_eq!(round_trip(&operation), operation);
    let event = Event {
        kind: EventKind::OperationCompleted,
        entity: None,
        revision: None,
        snapshot: None,
        operation: Some(OperationReference {
            id: "op-1".to_owned(),
        }),
        metadata: vec![("source".to_owned(), "fixture".to_owned())],
    };
    assert_eq!(round_trip(&event), event);
}

/// Verifies protocol version metadata remains explicit and stable.
#[test]
fn protocol_version_is_explicit() {
    let version = ProtocolVersion::default();
    assert_eq!(version.major, 1);
    assert_eq!(round_trip(&version), version);
}
