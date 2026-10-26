//! Covers additional focused cases from the parent test module.

use super::*;

#[expect(
    clippy::too_many_lines,
    reason = "This protocol contract regression verifies the complete model request and presentation payload family in one round-trip matrix."
)]
/// Verifies model, prompt, and presentation contracts round trip.
pub(super) fn model_contracts_round_trip() {
    round_trip(&ModelRequestDetailRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        branch: BranchReference {
            id: "branch-1".to_owned(),
        },
        invocation: "invocation-1".to_owned(),
        offset: None,
        max_chars: None,
    });
    round_trip(&ModelRequestDetailResponse {
        invocation: "invocation-1".to_owned(),
        model: "model-1".to_owned(),
        provider_id: Some("provider-1".to_owned()),
        provider_profile: None,
        thinking_level: Some("careful".to_owned()),
        profile: Some(ModelContextRevisionReference {
            identity: "profile.default".to_owned(),
            revision: "rev-2".to_owned(),
        }),
        active_components: vec![ModelContextRevisionReference {
            identity: "component.core".to_owned(),
            revision: "rev-7".to_owned(),
        }],
        context_fingerprint: None,
        request_fingerprint: None,
        catalog_revision: None,
        upstream_runtime_revision: None,
        provider_response_id: None,
        model_execution_configuration: None,
        messages: vec![ModelRequestMessageDetail {
            role: "user".to_owned(),
            content: "safe prompt".to_owned(),
        }],
        tools: vec![ModelRequestToolDetail {
            name: "inspect".to_owned(),
            description: "Inspect an entity".to_owned(),
            parameters: Some("{}".to_owned()),
        }],
        applied_settings: None,
        applied_settings_fingerprint: None,
        truncated: false,
        content_complete: false,
        fragments: Vec::new(),
        offset: None,
        total_chars: None,
        next_offset: None,
    });
    let legacy_detail: ModelRequestDetailResponse = serde_json::from_value(serde_json::json!({
        "invocation": "invocation-legacy",
        "model": "model-legacy",
        "thinking_level": null,
        "messages": [],
        "tools": [],
        "truncated": false
    }))
    .expect("legacy model request detail");
    assert_eq!(legacy_detail.provider_id, None);
    assert_eq!(legacy_detail.profile, None);
    assert_eq!(legacy_detail.active_components, Vec::new());
    round_trip(&ModelLibraryRequest::default());
    round_trip(&SessionModelSelectionRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        expected_revision: 7,
        provider_id: "provider-1".to_owned(),
        model_id: "model-1".to_owned(),
    });
    round_trip(&SessionThinkingSelectionRequest {
        session: SessionReference {
            id: "session-1".to_owned(),
        },
        expected_revision: 8,
        thinking_level: "extended".to_owned(),
    });
    round_trip(&LegacyExecutionRecordsRequest {
        session: SessionReference {
            id: "session-legacy".to_owned(),
        },
    });
    round_trip(&LegacyExecutionMappingRequest {
        session: SessionReference {
            id: "session-legacy".to_owned(),
        },
        expected_revision: 9,
        legacy_path: "/execution".to_owned(),
        provider_id: "provider-1".to_owned(),
        model_id: "model-1".to_owned(),
        thinking_level: None,
    });
    round_trip(&LegacyExecutionRecordView {
        path: "/execution".to_owned(),
        value: serde_json::json!({"model": "retained"}),
        target: Some(LegacyExecutionTargetView {
            plan_id: "plan-1".to_owned(),
            node_id: "node-1".to_owned(),
            revision: Some(2),
            claim: false,
        }),
    });
    round_trip(&Request::LegacyExecutionRecords(
        LegacyExecutionRecordsRequest {
            session: SessionReference {
                id: "session-legacy".to_owned(),
            },
        },
    ));
    round_trip(&Request::LegacyExecutionMapping(
        LegacyExecutionMappingRequest {
            session: SessionReference {
                id: "session-legacy".to_owned(),
            },
            expected_revision: 2,
            legacy_path: "/execution".to_owned(),
            provider_id: "provider-1".to_owned(),
            model_id: "model-1".to_owned(),
            thinking_level: None,
        },
    ));
    round_trip(&Request::LegacyPlanAssignmentMapping(
        LegacyPlanAssignmentMappingRequest {
            session: SessionReference {
                id: "session-legacy".to_owned(),
            },
            legacy_path: "/plans/0/nodes/0/assignment".to_owned(),
            expected_plan_revision: 4,
            provider_id: "provider-1".to_owned(),
            model_id: "model-1".to_owned(),
            thinking_level: None,
        },
    ));
    round_trip(&Response::LegacyExecutionRecords(vec![
        LegacyExecutionRecordView {
            path: "/execution".to_owned(),
            value: serde_json::json!({"model": "legacy"}),
            target: None,
        },
    ]));
    round_trip(&ModelLibraryResponse {
        provider_readiness: Vec::new(),
        models: vec![PublicModelDescriptor {
            id: model_row_id("provider-1", "provider-model-1"),
            display_name: "Safe Model Name".to_owned(),
            provider_id: "provider-1".to_owned(),
            provider_model_id: "provider-model-1".to_owned(),
            thinking_levels: vec!["deep-dive".to_owned()],
            metadata_snapshot: model_metadata_snapshot(),
        }],
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
