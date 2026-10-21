//! Verifies `ScribeRig` provenance captured from upstream request values.

use rpi_ai::providers::faux::{FauxProvider, FauxScript};
use rpi_ai::types::{Context, Message, ThinkingLevel, UserMessage};

use crate::context::{BranchExecutionContext, ContextCompiler, ContextPolicy};

use super::capture_provider_request;

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
