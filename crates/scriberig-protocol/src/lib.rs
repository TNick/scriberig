//! Defines stable serializable transport contracts without server business logic.

pub mod capability;
pub mod error;
pub mod event;
pub mod operation;
pub mod refs;
pub mod request;
pub mod response;
pub mod version;

pub use capability::{CapabilitiesResponse, CapabilityInfo, RuntimeFeature};
pub use error::{ProtocolError, ProtocolErrorCode};
pub use event::{Event, EventKind};
pub use operation::{Operation, OperationProgress, OperationStatus};
pub use refs::{EntityReference, OperationReference, RevisionReference, SnapshotReference};
pub use request::{
    CancelOperationRequest, InspectEntityRequest, RuntimeCapabilitiesRequest, SnapshotSelector,
    SubscribeRequest,
};
pub use response::{InspectEntityResponse, OperationStatusResponse};
pub use version::{ProtocolVersion, VersionExchange};

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    /// Verifies a value survives JSON encoding without semantic loss.
    fn round_trip<T>(value: &T)
    where
        T: DeserializeOwned + std::fmt::Debug + PartialEq + Serialize,
    {
        let encoded = serde_json::to_string(value).expect("serialize protocol value");
        let decoded: T = serde_json::from_str(&encoded).expect("deserialize protocol value");
        assert_eq!(decoded, *value);
    }

    /// Verifies stable references and snapshot-aware requests round trip.
    #[test]
    fn references_and_requests_round_trip() {
        let entity = EntityReference {
            id: "entity-1".to_owned(),
        };
        round_trip(&entity);
        round_trip(&InspectEntityRequest {
            entity,
            snapshot: SnapshotSelector::Current,
            detail: request::DetailLevel::Deep,
        });
    }

    /// Verifies errors preserve every machine-readable category field.
    #[test]
    fn structured_error_round_trip() {
        round_trip(&ProtocolError {
            code: ProtocolErrorCode::PermissionDenied,
            message: "denied".to_owned(),
            details: Some(serde_json::json!({"action": "inspect"})),
            correlation_id: Some("request-1".to_owned()),
        });
    }

    /// Verifies operation, event, and version values round trip together.
    #[test]
    fn operation_event_and_version_round_trip() {
        round_trip(&VersionExchange {
            active: ProtocolVersion::default(),
            supported: vec![ProtocolVersion::default()],
        });
        round_trip(&Operation {
            reference: OperationReference {
                id: "op-1".to_owned(),
            },
            kind: operation::OperationKind::Inspect,
            status: operation::OperationStatus::Completed,
            progress: Some(operation::OperationProgress {
                percent: 100,
                message: Some("done".to_owned()),
            }),
            result_snapshot: None,
            error: None,
        });
        round_trip(&Event {
            kind: EventKind::EntityChanged,
            entity: Some(EntityReference {
                id: "entity-1".to_owned(),
            }),
            revision: None,
            snapshot: None,
            operation: None,
            metadata: vec![("source".to_owned(), "test".to_owned())],
        });
    }
}
