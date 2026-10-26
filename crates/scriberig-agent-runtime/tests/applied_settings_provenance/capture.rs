//! Captures upstream applied-settings reports at the provider boundary.

use std::sync::{Arc, Mutex};

use rpi_ai::AppliedRequestSettings;
use scriberig_agent_runtime::upstream_invocation::run_upstream_branch_invocation_blocking;

use super::fixture::{
    CREDENTIAL_SENTINEL, ReportMode, SettingsProvider, fixture_model, invocation,
};

/// Captures one attested upstream report and keeps its omissions and status.
#[test]
fn attested_report_is_captured_with_explicit_omissions_and_completeness() {
    let requests = super::fixture::capture(ReportMode::Attested);
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    let report = request
        .applied_settings
        .as_ref()
        .expect("report captured from the final provider boundary");
    assert_eq!(
        report["schemaVersion"],
        serde_json::json!("rpi-applied-request-settings-v1")
    );
    assert_eq!(
        report["providerId"],
        serde_json::json!("fixture-applied-settings")
    );
    assert_eq!(
        report["modelId"],
        serde_json::json!("applied-settings-model")
    );
    assert_eq!(
        report["apiId"],
        serde_json::json!("applied-settings-fixture")
    );
    assert_eq!(
        report["appliedValues"]["temperature"],
        serde_json::json!(0.25)
    );
    assert_eq!(
        report["appliedValues"]["reasoning"]["effort"],
        serde_json::json!("medium")
    );
    assert_eq!(report["complete"], serde_json::json!(false));
    for category in ["headers_and_credentials", "prompt_and_transcript"] {
        assert!(
            report["omittedCategories"]
                .as_array()
                .expect("omitted categories")
                .iter()
                .any(|value| value.as_str() == Some(category)),
            "an incomplete report must name the omitted {category} category"
        );
    }
    let fingerprint = request
        .applied_settings_fingerprint
        .as_deref()
        .expect("versioned report digest");
    assert!(fingerprint.len() == 64 && fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(request.request_fingerprint.is_some());
    assert!(
        !serde_json::to_string(report)
            .expect("serialize report")
            .contains(CREDENTIAL_SENTINEL),
        "the upstream report must not carry catalog credentials"
    );
}

/// Captures versioned provider-body and safe-header identity.
#[test]
fn versioned_report_identity_is_captured_and_changes_request_identity() {
    let version_one = super::fixture::capture(ReportMode::Attested);
    let version_two = super::fixture::capture(ReportMode::AttestedV2);
    let version_three = super::fixture::capture(ReportMode::AttestedV3);
    let report = version_two[0]
        .applied_settings
        .as_ref()
        .expect("version-2 report captured");

    assert_eq!(
        report["schemaVersion"],
        serde_json::json!("rpi-applied-request-settings-v2")
    );
    assert_eq!(
        report["requestBodySha256"],
        serde_json::json!("a".repeat(64))
    );
    assert_ne!(
        version_one[0].request_fingerprint,
        version_two[0].request_fingerprint
    );
    let report = version_three[0]
        .applied_settings
        .as_ref()
        .expect("version-three report captured");
    assert_eq!(
        report["schemaVersion"],
        serde_json::json!("rpi-applied-request-settings-v3")
    );
    assert_eq!(
        report["requestHeaderSha256"],
        serde_json::json!("b".repeat(64))
    );
    assert_ne!(
        version_two[0].request_fingerprint,
        version_three[0].request_fingerprint
    );
}

/// Captures v4 option evidence only when it matches the selected model schema.
#[test]
fn version_four_option_evidence_is_schema_bound_and_secret_free() {
    let valid = super::fixture::capture(ReportMode::AttestedV4);
    let report = valid[0]
        .applied_settings
        .as_ref()
        .expect("valid v4 report captured");
    assert_eq!(
        report["schemaVersion"],
        serde_json::json!("rpi-applied-request-settings-v4")
    );
    assert_eq!(
        report["providerOptions"]["options"]
            .as_array()
            .expect("provider options")
            .len(),
        1
    );
    assert_eq!(
        report["providerOptions"]["omittedCount"],
        serde_json::json!(0)
    );
    let encoded = serde_json::to_string(report).expect("serialize report");
    assert!(!encoded.contains("future_vendor_mode"));
    assert!(!encoded.contains("fast"));

    let transformed = super::fixture::capture(ReportMode::AttestedV4Transformed);
    let transformed_report = transformed[0]
        .applied_settings
        .as_ref()
        .expect("transformed v4 report captured");
    assert_eq!(
        transformed_report["providerOptions"]["options"][0]["disposition"],
        serde_json::json!("transformed")
    );
    assert_ne!(
        valid[0].request_fingerprint,
        transformed[0].request_fingerprint
    );

    let invalid = super::fixture::capture(ReportMode::InvalidV4);
    assert!(invalid[0].applied_settings.is_none());
}

/// Captures v5 only when its exact final-request digest is valid.
#[test]
fn version_five_effect_identity_requires_a_valid_digest() {
    let valid = super::fixture::capture(ReportMode::AttestedV5);
    let report = valid[0]
        .applied_settings
        .as_ref()
        .expect("valid v5 report captured");
    assert_eq!(
        report["schemaVersion"],
        serde_json::json!("rpi-applied-request-settings-v5")
    );
    assert_eq!(
        report["requestEffectSha256"],
        serde_json::json!("c".repeat(64))
    );
    assert_eq!(report["omittedCategories"], serde_json::json!([]));
    assert_ne!(
        valid[0].request_fingerprint,
        super::fixture::capture(ReportMode::AttestedV4)[0].request_fingerprint
    );

    let partial = super::fixture::capture(ReportMode::PartialV5);
    let partial_report = partial[0]
        .applied_settings
        .as_ref()
        .expect("incomplete v5 report captured");
    assert_eq!(partial_report["complete"], serde_json::json!(false));
    assert!(partial_report.get("requestEffectSha256").is_none());
    assert!(
        partial_report["omittedCategories"]
            .as_array()
            .expect("incomplete categories")
            .iter()
            .any(|category| category == "request_effect_identity")
    );

    let invalid = super::fixture::capture(ReportMode::InvalidV5);
    assert!(invalid[0].applied_settings.is_none());
    assert!(invalid[0].applied_settings_fingerprint.is_none());

    let contradictory = super::fixture::capture(ReportMode::CompleteWithOmissionsV5);
    assert!(contradictory[0].applied_settings.is_none());
    assert!(contradictory[0].applied_settings_fingerprint.is_none());
}

/// Rejects reports that do not attest the exact bound request.
#[test]
fn reports_outside_the_bound_request_are_not_captured() {
    for mode in [
        ReportMode::OtherModel,
        ReportMode::OtherProvider,
        ReportMode::OtherApi,
        ReportMode::UnknownSchema,
        ReportMode::InvalidV2,
        ReportMode::InvalidV3,
        ReportMode::InvalidV4,
        ReportMode::CompleteWithOmissionsV5,
        ReportMode::Unattested,
    ] {
        let requests = super::fixture::capture(mode);
        assert_eq!(requests.len(), 1);
        assert!(
            requests[0].applied_settings.is_none(),
            "a report that does not attest the bound provider/model/API must be rejected"
        );
        assert!(
            requests[0].applied_settings_fingerprint.is_none(),
            "a rejected report must not contribute a provenance digest"
        );
    }
}

/// Keeps catalog metadata out of the applied-settings report boundary.
#[test]
fn catalog_metadata_is_never_reported_as_applied() {
    let requests = super::fixture::capture(ReportMode::Silent);
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert!(
        request.model_execution_configuration.is_some(),
        "the safe catalog-derived execution configuration is separate provenance"
    );
    assert!(
        request.applied_settings.is_none(),
        "catalog metadata alone is never reported as applied request settings"
    );
    assert!(request.applied_settings_fingerprint.is_none());
}

/// Forwards only a validated report to the host observer.
#[test]
fn the_host_observer_receives_only_a_validated_report() {
    for (mode, expected) in [
        (ReportMode::Attested, 1_usize),
        (ReportMode::AttestedV2, 1),
        (ReportMode::AttestedV3, 1),
        (ReportMode::OtherModel, 0),
    ] {
        let received: Arc<Mutex<Vec<AppliedRequestSettings>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&received);
        let callback: rpi_ai::AppliedRequestSettingsCallback = Arc::new(move |report| {
            if let Ok(mut reports) = sink.lock() {
                reports.push(report);
            }
        });
        let provider_id = "fixture-applied-settings";
        let model = fixture_model(provider_id, "applied-settings-model");
        let provider = Arc::new(SettingsProvider::new(
            provider_id,
            mode,
            vec![model.clone()],
        ));
        run_upstream_branch_invocation_blocking(
            invocation(provider as Arc<dyn rpi_ai::Provider>, model, Some(callback)),
            "review the bounded request",
        )
        .expect("upstream branch invocation");
        let reports = received.lock().expect("observer lock").clone();
        assert_eq!(
            reports.len(),
            expected,
            "the host observer must mirror capture for {mode:?}",
        );
        if expected == 1 {
            assert!(matches!(
                reports[0].schema_version.as_str(),
                "rpi-applied-request-settings-v1"
                    | "rpi-applied-request-settings-v2"
                    | "rpi-applied-request-settings-v3"
            ));
            assert_eq!(reports[0].model_id, "applied-settings-model");
            assert!(!reports[0].complete);
            assert!(
                reports[0]
                    .omitted_categories
                    .iter()
                    .any(|category| category == "headers_and_credentials"),
                "a partial report must keep its omitted categories when forwarded"
            );
        }
    }
}

/// Keeps the captured full prompt as the complete safe `ScribeRig` context.
///
/// The upstream applied-settings report is a separate observation of the
/// serialized provider request. The full prompt must stay the compiled
/// `ScribeRig` context and must not be reconstructed from request fields.
#[test]
fn the_full_prompt_is_the_compiled_context_not_a_serialized_request_body() {
    let compiled = super::fixture::compiled_context();
    let requests = super::fixture::capture(ReportMode::Attested);
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert!(
        request.content_complete,
        "a text-only compiled context must be captured completely"
    );
    assert!(!request.truncated);
    let full = request
        .full_messages
        .as_ref()
        .expect("complete safe message projection");
    assert_eq!(
        full.iter()
            .filter(|message| message.role == "system")
            .count(),
        1,
        "the compiled provider context is captured as the system section"
    );
    let captured_text = full
        .iter()
        .map(|message| {
            message
                .parts
                .iter()
                .map(|part| match part {
                    scriberig_session::ModelRequestDetailPart::Text(text) => text.as_str(),
                    scriberig_session::ModelRequestDetailPart::ToolCall(call) => call.name.as_str(),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let system_sections = full
        .iter()
        .filter(|message| message.role == "system")
        .flat_map(|message| message.parts.iter())
        .collect::<Vec<_>>();
    assert_eq!(system_sections.len(), 1);
    let scriberig_session::ModelRequestDetailPart::Text(system_text) = &system_sections[0] else {
        panic!("the provider context section must be captured as text");
    };
    assert_eq!(
        system_text, &compiled.provider_context,
        "the provider-visible compiled context is captured exactly, not re-derived"
    );
    assert_eq!(
        full.iter()
            .filter(|message| message.role == "user")
            .flat_map(|message| message.parts.iter())
            .count(),
        1,
        "the current instruction is delivered once in the full prompt"
    );
    assert!(
        captured_text.contains("review the bounded request"),
        "the current instruction must be part of the full prompt"
    );
    assert!(
        captured_text.contains("purpose:applied settings provenance proof"),
        "the compiled branch purpose must reach the provider context section"
    );
    let prompt_json = serde_json::to_string(&request.full_messages).expect("serialize prompt");
    for request_field in [
        "max_tokens",
        "temperature",
        "previous_response_id",
        "stream",
    ] {
        assert!(
            !prompt_json.contains(request_field),
            "the full prompt is the safe context, not the serialized request body"
        );
    }
    let report = request
        .applied_settings
        .as_ref()
        .expect("captured upstream report");
    assert!(
        serde_json::to_string(report)
            .expect("serialize report")
            .contains("temperature"),
        "provider-applied request fields stay in the separate applied-settings report"
    );
}
