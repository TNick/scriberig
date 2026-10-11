//! Verifies public protocol values round trip without relying on Debug output.

use scriberig_protocol::{
    CancelOperationRequest, EntityReference, Event, EventKind, InspectEntityRequest, Operation,
    OperationProgress, OperationReference, ProtocolError, ProtocolErrorCode, ProtocolVersion,
    Request, RequestEnvelope, SnapshotReference, SnapshotSelector, SubscribeRequest,
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
        result_handles: Vec::new(),
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

/// Verifies cancellation and event subscription requests retain their filters.
#[test]
fn operation_control_requests_round_trip() {
    let cancel = Request::CancelOperation(CancelOperationRequest {
        operation: OperationReference {
            id: "op-1".to_owned(),
        },
    });
    let subscribe = Request::Subscribe(SubscribeRequest {
        event_kinds: vec!["EntityChanged".to_owned()],
    });
    assert_eq!(round_trip(&cancel), cancel);
    assert_eq!(round_trip(&subscribe), subscribe);
}

/// Verifies protocol version metadata remains explicit and stable.
#[test]
fn protocol_version_is_explicit() {
    let version = ProtocolVersion::default();
    assert_eq!(version.major, 1);
    assert_eq!(round_trip(&version), version);
}

/// Verifies additive JSON fields do not break existing public messages.
#[test]
fn additive_fields_are_ignored_by_older_readers() {
    let value = serde_json::json!({
        "reference": {"id": "op-1"},
        "kind": "Inspect",
        "status": "Completed",
        "progress": null,
        "result_snapshot": null,
        "error": null,
        "future_result": {"entity": "ent-1"}
    });
    let operation: Operation = serde_json::from_value(value).expect("additive field");
    assert_eq!(operation.reference.id, "op-1");
    assert_eq!(
        operation.status,
        scriberig_protocol::operation::OperationStatus::Completed
    );
}
