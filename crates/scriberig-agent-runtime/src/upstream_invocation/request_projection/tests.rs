//! Verifies `ScribeRig` provenance captured from upstream request values.

use rpi_ai::providers::faux::{FauxProvider, FauxScript};
use rpi_ai::types::{Context, Message, ThinkingLevel, UserMessage};

use crate::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};

use super::capture_provider_request;

/// Ensures image bytes affect identity without entering durable provenance.
#[test]
fn provider_image_content_changes_fingerprint_without_persisting_image_data() {
    let provider = FauxProvider::new(FauxScript::new());
    let model = provider.default_model().clone();
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(
        &BranchExecutionContext::new("session", "branch", "goal", "safe prompt"),
    );
    let context_with_image = |data: &str| {
        Context::new(vec![Message::User(UserMessage::new(
            rpi_ai::types::UserContent::Blocks(vec![
                rpi_ai::types::Content::text("safe prompt"),
                rpi_ai::types::Content::Image(rpi_ai::types::ImageContent {
                    kind: rpi_ai::types::ImageContentType,
                    data: data.to_owned(),
                    mime_type: "image/png".to_owned(),
                }),
            ]),
            1,
        ))])
    };
    let first = capture_provider_request(
        &model,
        &context_with_image("private-image-payload-a"),
        None,
        &compiled,
        ThinkingLevel::Off,
    );
    let second = capture_provider_request(
        &model,
        &context_with_image("private-image-payload-b"),
        None,
        &compiled,
        ThinkingLevel::Off,
    );

    assert_ne!(first.request_fingerprint, second.request_fingerprint);
    let serialized = serde_json::to_string(&first).expect("request projection serializes");
    assert!(!serialized.contains("private-image-payload-a"));
}

/// Keeps provider-profile provenance outside upstream request content.
#[test]
fn provider_profile_changes_provenance_fingerprint_only() {
    let provider = FauxProvider::new(FauxScript::new());
    let model = provider.default_model().clone();
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(
        &BranchExecutionContext::new("session", "branch", "goal", "safe prompt"),
    );
    let context = Context::new(vec![Message::User(UserMessage::new("safe prompt", 1))]);

    let first = capture_provider_request(
        &model,
        &context,
        Some("scriberig-profile-a"),
        &compiled,
        ThinkingLevel::Off,
    );
    let second = capture_provider_request(
        &model,
        &context,
        Some("scriberig-profile-b"),
        &compiled,
        ThinkingLevel::Off,
    );

    assert_eq!(
        first.provider_profile.as_deref(),
        Some("scriberig-profile-a")
    );
    assert_eq!(
        second.provider_profile.as_deref(),
        Some("scriberig-profile-b")
    );
    assert_ne!(first.request_fingerprint, second.request_fingerprint);
    assert!(
        first
            .messages
            .iter()
            .all(|message| !message.content.contains("scriberig-profile-a"))
    );
}

/// Captures upstream execution-affecting properties without transport secrets.
#[test]
fn model_execution_properties_change_identity_without_secrets() {
    let provider = FauxProvider::new(FauxScript::new());
    let model = provider.default_model().clone();
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(
        &BranchExecutionContext::new("session", "branch", "goal", "safe prompt"),
    );
    let context = Context::new(vec![Message::User(UserMessage::new("safe prompt", 1))]);

    let base = capture_provider_request(&model, &context, None, &compiled, ThinkingLevel::Off);

    let mut sampling = model.clone();
    sampling.sampling_params = Some(serde_json::Map::from_iter([(
        "temperature".to_owned(),
        serde_json::json!(0.25),
    )]));
    let sampling =
        capture_provider_request(&sampling, &context, None, &compiled, ThinkingLevel::Off);

    let mut compatibility = model.clone();
    compatibility.compat = Some(rpi_ai::StreamingProtocolCompat::OpenaiCompletions(
        rpi_ai::model::OpenaiCompletionsCompat {
            supports_strict_mode: Some(true),
            ..rpi_ai::model::OpenaiCompletionsCompat::default()
        },
    ));
    let compatibility = capture_provider_request(
        &compatibility,
        &context,
        None,
        &compiled,
        ThinkingLevel::Off,
    );

    let mut endpoint_model = model.clone();
    endpoint_model.base_url =
        "https://user:transport-secret@example.invalid/v1?token=transport-secret".to_owned();
    let endpoint = capture_provider_request(
        &endpoint_model,
        &context,
        None,
        &compiled,
        ThinkingLevel::Off,
    );
    let mut changed_unsafe_endpoint = endpoint_model.clone();
    changed_unsafe_endpoint.base_url =
        "https://user:changed-secret@example.invalid/v1?token=changed-secret".to_owned();
    let changed_unsafe_endpoint = capture_provider_request(
        &changed_unsafe_endpoint,
        &context,
        None,
        &compiled,
        ThinkingLevel::Off,
    );

    let mut safe_endpoint = model.clone();
    safe_endpoint.base_url = "https://gateway.example.invalid/v2".to_owned();
    let safe_endpoint = capture_provider_request(
        &safe_endpoint,
        &context,
        None,
        &compiled,
        ThinkingLevel::Off,
    );

    let mut headers = model.clone();
    headers.headers = Some(std::collections::BTreeMap::from([(
        "authorization".to_owned(),
        "transport-secret".to_owned(),
    )]));
    let headers = capture_provider_request(&headers, &context, None, &compiled, ThinkingLevel::Off);

    assert_ne!(base.request_fingerprint, sampling.request_fingerprint);
    assert_ne!(base.request_fingerprint, compatibility.request_fingerprint);
    assert_ne!(base.request_fingerprint, endpoint.request_fingerprint);
    assert_eq!(
        endpoint.request_fingerprint,
        changed_unsafe_endpoint.request_fingerprint
    );
    assert_ne!(base.request_fingerprint, safe_endpoint.request_fingerprint);
    assert!(
        !serde_json::to_string(&safe_endpoint)
            .expect("request projection serializes")
            .contains("gateway.example.invalid")
    );
    assert!(
        !serde_json::to_string(&endpoint)
            .expect("request projection serializes")
            .contains("transport-secret")
    );
    assert!(
        !serde_json::to_string(&headers)
            .expect("request projection serializes")
            .contains("transport-secret")
    );
}

/// Verifies an invocation-level selection override changes captured provenance
/// without changing the profile reference used for the invocation.
///
/// The narrower invocation scope supplies a different exact upstream model
/// while the compiled context keeps the same profile identity and revision, so
/// the historical request identity moves and the profile reference does not.
#[test]
fn invocation_model_override_changes_provenance_without_changing_the_profile() {
    let provider = FauxProvider::new(FauxScript::new());
    let profile_model = provider.default_model().clone();
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(
        &BranchExecutionContext::new("session", "branch", "goal", "safe prompt"),
    );
    let context = Context::new(vec![Message::User(UserMessage::new("safe prompt", 1))]);
    let mut override_model = profile_model.clone();
    override_model.id = "invocation-override-model".to_owned();

    let preferred = capture_provider_request(
        &profile_model,
        &context,
        None,
        &compiled,
        ThinkingLevel::High,
    );
    let overridden = capture_provider_request(
        &override_model,
        &context,
        None,
        &compiled,
        ThinkingLevel::High,
    );

    assert_eq!(preferred.model, profile_model.id);
    assert_eq!(overridden.model, "invocation-override-model");
    assert_eq!(preferred.thinking_level, overridden.thinking_level);
    assert!(
        preferred.profile.is_some(),
        "the captured profile reference must remain exact"
    );
    assert_eq!(preferred.profile, overridden.profile);
    assert_ne!(
        preferred.request_fingerprint, overridden.request_fingerprint,
        "an invocation override must move the captured request identity"
    );
}
