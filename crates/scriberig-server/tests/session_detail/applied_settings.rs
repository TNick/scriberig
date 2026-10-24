//! Round-trips one upstream applied-settings report through the protocol detail
//! operation and proves catalog metadata alone is never reported as applied.

use scriberig_app::{
    ApplicationContext, ApplicationService, CapabilityContext, CapabilityId, SessionBackend,
};

use super::fixture::{Fixture, dispatch_detail, projection_with_settings};

/// Keeps the upstream report shape, omissions, and completeness in the API read.
#[test]
fn applied_settings_report_round_trips_through_the_protocol_detail_operation() {
    let report = serde_json::to_value(rpi_ai::AppliedRequestSettings {
        schema_version: "rpi-applied-request-settings-v1".to_owned(),
        provider_id: "provider-a".to_owned(),
        model_id: "model-a".to_owned(),
        api_id: "openai-completions".to_owned(),
        applied_values: std::collections::BTreeMap::from([(
            "temperature".to_owned(),
            serde_json::json!(0.25),
        )]),
        omitted_categories: vec![
            "endpoint".to_owned(),
            "headers_and_credentials".to_owned(),
            "prompt_and_transcript".to_owned(),
        ],
        complete: false,
    })
    .expect("serialize the upstream report");
    let service = ApplicationService::new(Fixture);
    let handle = service
        .start_local_agent(&scriberig_app::AgentCommandRequest {
            session: scriberig_core::EntityId::from_bytes([35; 16]),
            branch: None,
            anchor: None,
            prompt: "inspect safely".to_owned(),
            selection: None,
        })
        .expect("start local agent");
    service
        .complete_local_agent_with_evidence(
            &handle,
            "done",
            Vec::new(),
            Vec::new(),
            vec![projection_with_settings(
                "model-a",
                Some(report.clone()),
                Some(serde_json::json!({ "excludedExecutionFields": ["endpoint"] })),
            )],
        )
        .expect("persist the upstream report");
    let read_context = ApplicationContext {
        capabilities: CapabilityContext::new(vec![
            CapabilityId::new("session_branch_tree").expect("session read capability"),
        ]),
        ..ApplicationContext::empty()
    };
    let branch = service
        .read_session(
            &read_context,
            &scriberig_app::SessionRequest {
                session: handle.session,
            },
        )
        .expect("read durable session")
        .branches[0]
        .clone();

    let detail = dispatch_detail(
        &service,
        handle.session,
        handle.root_branch,
        branch.model_invocations[0].id,
    );
    assert_eq!(
        detail.applied_settings.as_ref(),
        Some(&report),
        "the protocol detail read must return the report captured at the provider boundary"
    );
    assert_eq!(
        detail
            .applied_settings
            .as_ref()
            .and_then(|value| value.get("complete"))
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "the report must stay explicitly incomplete across the transport"
    );
    assert_eq!(
        detail
            .applied_settings
            .as_ref()
            .and_then(|value| value.get("omittedCategories"))
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len),
        3,
        "the report must keep its explicit omissions across the transport"
    );
    assert_ne!(
        detail.applied_settings, detail.model_execution_configuration,
        "catalog-derived execution metadata is a separate observation"
    );
}

/// Reports no applied settings when only catalog metadata exists.
#[test]
fn catalog_metadata_alone_is_not_reported_as_applied_settings() {
    let service = ApplicationService::new(Fixture);
    let handle = service
        .start_local_agent(&scriberig_app::AgentCommandRequest {
            session: scriberig_core::EntityId::from_bytes([36; 16]),
            branch: None,
            anchor: None,
            prompt: "inspect safely".to_owned(),
            selection: None,
        })
        .expect("start local agent");
    service
        .complete_local_agent_with_evidence(
            &handle,
            "done",
            Vec::new(),
            Vec::new(),
            vec![projection_with_settings(
                "model-a",
                None,
                Some(serde_json::json!({
                    "resolvedModel": "model-a",
                    "thinkingLevels": [{ "label": "low" }],
                    "excludedExecutionFields": ["endpoint", "headers"]
                })),
            )],
        )
        .expect("persist catalog provenance only");
    let read_context = ApplicationContext {
        capabilities: CapabilityContext::new(vec![
            CapabilityId::new("session_branch_tree").expect("session read capability"),
        ]),
        ..ApplicationContext::empty()
    };
    let branch = service
        .read_session(
            &read_context,
            &scriberig_app::SessionRequest {
                session: handle.session,
            },
        )
        .expect("read durable session")
        .branches[0]
        .clone();

    let detail = dispatch_detail(
        &service,
        handle.session,
        handle.root_branch,
        branch.model_invocations[0].id,
    );
    assert!(detail.applied_settings.is_none());
    assert!(detail.applied_settings_fingerprint.is_none());
    assert!(detail.model_execution_configuration.is_some());
}
