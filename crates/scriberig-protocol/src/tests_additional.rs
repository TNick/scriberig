//! Covers additional focused cases from the parent test module.

use super::*;

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
        agent_result: None,
        error: None,
    });
    round_trip(&Event {
        sequence: 1,
        kind: EventKind::EntityChanged,
        entity: Some(EntityReference {
            id: "entity-1".to_owned(),
        }),
        revision: None,
        snapshot: None,
        operation: None,
        activity_kind: None,
        activity_state: None,
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
        sequence: 2,
        kind: EventKind::SnapshotPromoted,
        entity: Some(selection.entity),
        revision: None,
        snapshot: Some(selection.snapshot),
        operation: Some(OperationReference {
            id: "op-1".to_owned(),
        }),
        activity_kind: None,
        activity_state: None,
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
