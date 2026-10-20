//! Verifies safe model-request projection and fingerprints.

use super::capture;
use crate::context::CompiledContext;
use crate::provider::{ModelMessage, ModelRequest, ToolChoice};
use scriberig_agent_core::{AgentProfileRevisionRef, ContextComponentRevisionRef};

/// Omits transport-only authorization metadata from public request detail.
#[test]
fn authorization_marker_is_absent_from_captured_projection() {
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert(
        "authorization".to_owned(),
        "FAKE_PROVIDER_SECRET_MARKER_8391".to_owned(),
    );
    let request = ModelRequest {
        model: "model-a".to_owned(),
        thinking_level: Some("careful".to_owned()),
        messages: vec![ModelMessage::user("safe prompt")],
        tools: Vec::new(),
        tool_choice: ToolChoice::Auto,
        response_schema: None,
        temperature: None,
        max_tokens: None,
        metadata,
    };

    let projection = capture(&request, None, None, None);
    let encoded = serde_json::to_string(&projection).expect("safe projection JSON");

    assert!(!encoded.contains("FAKE_PROVIDER_SECRET_MARKER_8391"));
    assert!(!encoded.contains("authorization"));
    assert!(encoded.contains("safe prompt"));
}

/// Marks captured prompt detail when its provider-neutral budget truncates.
#[test]
fn captured_model_request_detail_is_bounded_and_marks_truncation() {
    let request = ModelRequest::new("model-a", vec![ModelMessage::user("x".repeat(20_000))]);

    let projection = capture(&request, None, None, None);

    assert!(projection.truncated);
    assert_eq!(projection.messages[0].content.chars().count(), 15_000);
}

/// Retains exact context revisions and identity without parsing prompt text.
#[test]
fn captured_request_carries_compiled_context_provenance() {
    let request = ModelRequest::new("model-a", vec![ModelMessage::user("prompt")]);
    let context = CompiledContext {
        profile: AgentProfileRevisionRef {
            identity: "profile.default".to_owned(),
            revision: "rev-3".to_owned(),
        },
        active_components: vec![ContextComponentRevisionRef {
            identity: "component.core".to_owned(),
            revision: "rev-8".to_owned(),
        }],
        messages: Vec::new(),
        dependencies: Vec::new(),
        omitted_handles: Vec::new(),
        canonical: "canonical compiled context".to_owned(),
        provider_context: "provider-visible context".to_owned(),
    };

    let projection = capture(
        &request,
        Some("provider-a".to_owned()),
        None,
        Some(&context),
    );

    assert_eq!(
        projection.profile,
        Some(scriberig_session::ModelContextRevisionRef {
            identity: "profile.default".to_owned(),
            revision: "rev-3".to_owned(),
        })
    );
    assert_eq!(
        projection.active_components,
        vec![scriberig_session::ModelContextRevisionRef {
            identity: "component.core".to_owned(),
            revision: "rev-8".to_owned(),
        }]
    );
    assert!(projection.context_fingerprint.is_some());
    assert!(projection.request_fingerprint.is_some());
    let identical = capture(
        &request,
        Some("provider-a".to_owned()),
        None,
        Some(&context),
    );
    assert_eq!(
        projection.request_fingerprint,
        identical.request_fingerprint
    );

    let mut changed_revision = context.clone();
    changed_revision.active_components[0].revision = "rev-9".to_owned();
    let changed = capture(
        &request,
        Some("provider-a".to_owned()),
        None,
        Some(&changed_revision),
    );
    assert_ne!(projection.request_fingerprint, changed.request_fingerprint);

    let changed_request =
        ModelRequest::new("model-a", vec![ModelMessage::user("different prompt")]);
    let changed_call = capture(
        &changed_request,
        Some("provider-a".to_owned()),
        None,
        Some(&context),
    );
    assert_ne!(
        projection.request_fingerprint,
        changed_call.request_fingerprint
    );
}

/// Changes request identity for every normalized execution input.
#[test]
fn request_fingerprint_covers_selection_context_and_request_fields() {
    let request = fingerprint_fixture_request();
    let context = fingerprint_fixture_context();
    let baseline = request_fingerprint(&request, Some("provider-a"), &context);

    let provider_changed = request_fingerprint(&request, Some("provider-b"), &context);
    assert_ne!(baseline, provider_changed);

    let mut changed = request.clone();
    changed.thinking_level = Some("deep".to_owned());
    assert_ne!(
        baseline,
        request_fingerprint(&changed, Some("provider-a"), &context)
    );

    let mut changed = request.clone();
    changed.response_schema = Some(serde_json::json!({"type": "object"}));
    assert_ne!(
        baseline,
        request_fingerprint(&changed, Some("provider-a"), &context)
    );

    let mut changed = request.clone();
    changed.tool_choice = ToolChoice::Required("inspect".to_owned());
    assert_ne!(
        baseline,
        request_fingerprint(&changed, Some("provider-a"), &context)
    );

    let mut changed = request.clone();
    changed.temperature = Some(0.25);
    assert_ne!(
        baseline,
        request_fingerprint(&changed, Some("provider-a"), &context)
    );

    let mut non_finite_temperature = request.clone();
    non_finite_temperature.temperature = Some(f32::NAN);
    assert_ne!(
        baseline,
        request_fingerprint(&non_finite_temperature, Some("provider-a"), &context)
    );

    let mut changed = request.clone();
    changed.max_tokens = Some(1200);
    assert_ne!(
        baseline,
        request_fingerprint(&changed, Some("provider-a"), &context)
    );

    let mut changed_message = request.clone();
    changed_message.messages[0].content = "different prompt".to_owned();
    assert_ne!(
        baseline,
        request_fingerprint(&changed_message, Some("provider-a"), &context)
    );

    let mut changed_tool_call = request.clone();
    changed_tool_call.messages[1].tool_call_id = Some("tool-call-other".to_owned());
    assert_ne!(
        baseline,
        request_fingerprint(&changed_tool_call, Some("provider-a"), &context)
    );

    let mut changed = request.clone();
    changed.tools[0].parameters = serde_json::json!({"type": "string"});
    assert_ne!(
        baseline,
        request_fingerprint(&changed, Some("provider-a"), &context)
    );

    let mut changed = context.clone();
    changed.profile.revision = "rev-4".to_owned();
    assert_ne!(
        baseline,
        request_fingerprint(&request, Some("provider-a"), &changed)
    );

    let mut changed = context.clone();
    changed.canonical.push_str(" additional context");
    assert_ne!(
        baseline,
        request_fingerprint(&request, Some("provider-a"), &changed)
    );

    let mut changed = context.clone();
    changed.active_components[0].revision = "rev-9".to_owned();
    assert_ne!(
        baseline,
        request_fingerprint(&request, Some("provider-a"), &changed)
    );

    let mut transport_only = request.clone();
    transport_only.metadata.insert(
        "authorization".to_owned(),
        "FAKE_CREDENTIAL_SENTINEL_9274".to_owned(),
    );
    assert_eq!(
        baseline,
        request_fingerprint(&transport_only, Some("provider-a"), &context)
    );
}

/// Creates one normalized request with every fingerprinted request field.
///
/// # Returns
/// A fixture request populated with selection, context, and tool values.
fn fingerprint_fixture_request() -> ModelRequest {
    let mut request = ModelRequest::new(
        "model-a",
        vec![
            ModelMessage::user("prompt"),
            ModelMessage {
                role: crate::provider::MessageRole::Tool,
                content: "tool result".to_owned(),
                tool_call_id: Some("tool-call-1".to_owned()),
            },
        ],
    );
    request.thinking_level = Some("careful".to_owned());
    request.tools.push(crate::provider::ToolDefinition {
        name: "inspect".to_owned(),
        description: "Inspect one entity".to_owned(),
        parameters: serde_json::json!({"type": "object"}),
    });
    request.response_schema = Some(serde_json::json!({"type": "string"}));
    request.temperature = Some(0.5);
    request.max_tokens = Some(900);
    request
}

/// Creates exact profile and component context for fingerprint fixtures.
///
/// # Returns
/// A fixture context with stable profile and component revisions.
fn fingerprint_fixture_context() -> CompiledContext {
    CompiledContext {
        profile: AgentProfileRevisionRef {
            identity: "profile.default".to_owned(),
            revision: "rev-3".to_owned(),
        },
        active_components: vec![ContextComponentRevisionRef {
            identity: "component.core".to_owned(),
            revision: "rev-8".to_owned(),
        }],
        messages: Vec::new(),
        dependencies: Vec::new(),
        omitted_handles: Vec::new(),
        canonical: "canonical compiled context".to_owned(),
        provider_context: "provider-visible context".to_owned(),
    }
}

/// Returns the canonical request fingerprint for one fixture input set.
///
/// - `request`: Normalized provider request.
/// - `provider_id`: Resolved provider identity, when present.
/// - `context`: Compiled profile/component context.
///
/// # Returns
/// The request fingerprint captured for these inputs.
fn request_fingerprint(
    request: &ModelRequest,
    provider_id: Option<&str>,
    context: &CompiledContext,
) -> String {
    capture(request, provider_id.map(str::to_owned), None, Some(context))
        .request_fingerprint
        .expect("request fingerprint is always captured")
}
