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

/// Rejects reports that do not attest the exact bound request.
#[test]
fn reports_outside_the_bound_request_are_not_captured() {
    for mode in [
        ReportMode::OtherModel,
        ReportMode::OtherProvider,
        ReportMode::OtherApi,
        ReportMode::UnknownSchema,
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
    for (mode, expected) in [(ReportMode::Attested, 1_usize), (ReportMode::OtherModel, 0)] {
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
            assert_eq!(reports[0].schema_version, "rpi-applied-request-settings-v1");
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
