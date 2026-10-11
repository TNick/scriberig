//! Defines stable serializable transport contracts without server business logic.

pub mod capability;
pub mod error;
pub mod event;
pub mod operation;
pub mod refs;
pub mod request;
pub mod response;
pub mod transport;
pub mod trust;
pub mod version;

pub use capability::{CapabilitiesResponse, CapabilityInfo, RuntimeFeature};
pub use error::{ProtocolError, ProtocolErrorCode};
pub use event::{Event, EventKind};
pub use operation::{Operation, OperationProgress, OperationStatus};
pub use refs::{EntityReference, OperationReference, RevisionReference, SnapshotReference};
pub use request::{
    CancelOperationRequest, CommandRequest, InspectEntityRequest, OperationStatusRequest,
    QueryRequest, RuntimeCapabilitiesRequest, SnapshotSelector, SubscribeRequest,
};
pub use response::{InspectEntityResponse, OperationStatusResponse, QueryResponse};
pub use transport::{ProtocolTransport, Request, RequestEnvelope, Response, ResponseEnvelope};
pub use trust::{
    ApprovalSummary, CapabilityDenial, ConflictSummary, ExternalEffectState, ExternalEffectSummary,
    PolicyEvaluationSummary, PromotionResult, PromotionState, ProvenanceInput, TrustDetails,
    TrustSelection, TrustSource, TrustSummary, ValidationDiagnostic, ValidationStatus,
    ValidationSummary,
};
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
        round_trip(&QueryRequest {
            specification: serde_json::json!({"scope": "snapshot"}),
        });
        round_trip(&CommandRequest {
            kind: "apply".to_owned(),
            payload: serde_json::json!({"change": "change-1"}),
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
            result_handles: Vec::new(),
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

    /// Verifies trust outcomes and event references round trip distinctly.
    #[test]
    fn trust_outcomes_and_events_round_trip() {
        let selection = TrustSelection {
            entity: EntityReference {
                id: "entity-1".to_owned(),
            },
            snapshot: SnapshotReference {
                id: "snapshot-1".to_owned(),
            },
            revision: None,
        };
        round_trip(&CapabilityDenial {
            action: "promote".to_owned(),
            scope: "entity-1".to_owned(),
            reason: "approval required".to_owned(),
            alternatives: vec!["inspect".to_owned()],
            approval_possible: true,
        });
        round_trip(&TrustDetails {
            summary: TrustSummary {
                source: TrustSource::ModelGenerated,
                validation: ValidationStatus::Current,
                accepted_via: Some("approval-1".to_owned()),
                restricted: false,
            },
            provenance: vec![ProvenanceInput {
                selection: selection.clone(),
                relation: "derived-from".to_owned(),
            }],
            validation: Some(ValidationSummary {
                status: ValidationStatus::Failed,
                diagnostics: vec![ValidationDiagnostic {
                    code: "schema".to_owned(),
                    message: "invalid value".to_owned(),
                    location: Some("field".to_owned()),
                }],
                operation: None,
            }),
            policy: None,
            conflict: None,
        });
        round_trip(&Event {
            kind: EventKind::SnapshotPromoted,
            entity: Some(selection.entity),
            revision: None,
            snapshot: Some(selection.snapshot),
            operation: Some(OperationReference {
                id: "op-1".to_owned(),
            }),
            metadata: vec![("candidate".to_owned(), "snapshot-2".to_owned())],
        });
        round_trip(&PromotionResult {
            state: PromotionState::Stale,
            candidate: Some(SnapshotReference {
                id: "candidate-1".to_owned(),
            }),
            promoted_snapshot: None,
            reason: Some("base changed".to_owned()),
        });
        round_trip(&ExternalEffectSummary {
            handle: "receipt-1".to_owned(),
            state: ExternalEffectState::Unknown,
            operation: None,
        });
    }
}
