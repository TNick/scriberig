//! Fingerprints normalized provider request values and safe upstream metadata.

use rpi_ai::Model;
use scriberig_session::{
    ModelContextRevisionRef, ModelRequestDetailMessage, ModelRequestDetailTool,
};

/// Borrows values that define request and semantic context identity.
///
/// # Public members
/// None; this internal helper is not part of the crate API.
///
/// # Private members
/// - `model`: Exact upstream model selected for the invocation.
/// - `thinking_level`: Upstream thinking level selected for the invocation.
/// - `messages`: Provider-visible message inputs.
/// - `tools`: Upstream tool schemas supplied to the provider.
/// - `image_fingerprints`: Ordered digests for provider-visible image blocks.
/// - `provider_profile`: Safe `ScribeRig` provider profile, when selected.
/// - `profile`: Exact immutable `ScribeRig` profile revision.
/// - `active_components`: Exact active context component references.
/// - `context_fingerprint`: Fingerprint of compiled `ScribeRig` context.
pub(super) struct RequestFingerprintInput<'a> {
    /// Exact upstream model selected for this invocation.
    pub(super) model: &'a Model,
    /// Upstream thinking level selected for this invocation.
    pub(super) thinking_level: &'a str,
    /// Ordered safe provider-visible message parts.
    pub(super) messages: &'a [ModelRequestDetailMessage],
    /// Complete safe upstream tool definitions supplied to the provider.
    pub(super) tools: &'a [ModelRequestDetailTool],
    /// Ordered hashes of image bytes and media types in provider messages.
    pub(super) image_fingerprints: &'a [String],
    /// Safe `ScribeRig` provider profile identifier, if selected.
    pub(super) provider_profile: Option<&'a str>,
    /// Exact immutable `ScribeRig` profile revision.
    pub(super) profile: Option<&'a ModelContextRevisionRef>,
    /// Exact immutable references for active context components.
    pub(super) active_components: &'a [ModelContextRevisionRef],
    /// Fingerprint of the compiled `ScribeRig` context.
    pub(super) context_fingerprint: Option<&'a str>,
}

/// Folds the upstream-owned execution fingerprint into request identity.
///
/// The upstream digest covers safe typed model metadata, including
/// compatibility flags, output limits, and a digest of a validated endpoint.
/// It omits headers, sampling values, provider options, and thinking-map labels
/// when those are present, and reports incomplete coverage. This adds that
/// digest without defining a `ScribeRig` property schema.
///
/// # Parameters
/// - `request_fingerprint`: Existing request and model-config digest.
/// - `model`: Exact upstream model selected for this invocation.
///
/// # Returns
/// A combined digest, or `None` when inputs cannot be serialized.
pub(super) fn request_fingerprint_with_model_execution(
    request_fingerprint: Option<&str>,
    model: &Model,
) -> Option<String> {
    let request_fingerprint = request_fingerprint?;
    let execution_fingerprint = rpi_model_runtime::model_execution_fingerprint(model);
    let material = serde_json::to_vec(&(
        "scriberig-request-with-model-execution-v1",
        request_fingerprint,
        &execution_fingerprint,
    ))
    .ok()?;
    Some(blake3::hash(&material).to_hex().to_string())
}

/// Fingerprints normalized request inputs and semantic context.
///
/// This digest may be extended with upstream model configuration and applied
/// settings. Omitted secrets, opaque properties, and other fields mean it is
/// not a complete provider request-equivalence fingerprint.
///
/// # Parameters
/// - `input`: Exact model, request content, and semantic context data.
///
/// # Returns
/// The hex-encoded base request fingerprint.
pub(super) fn request_fingerprint_for(input: &RequestFingerprintInput<'_>) -> String {
    let profile_identity = input
        .profile
        .map(|reference| (&reference.identity, &reference.revision));
    let component_identity = input
        .active_components
        .iter()
        .map(|reference| (&reference.identity, &reference.revision))
        .collect::<Vec<_>>();
    let material = serde_json::to_vec(&(
        "scriberig-request-v4",
        Some(input.model.provider.as_str()),
        input.provider_profile,
        &input.model.id,
        Some(input.thinking_level),
        ("auto", None::<&str>),
        &None::<serde_json::Value>,
        None::<u32>,
        None::<u32>,
        profile_identity,
        component_identity,
        input.context_fingerprint,
        input.messages,
        input.tools,
        input.image_fingerprints,
    ))
    .expect("request fingerprint inputs are serializable");
    blake3::hash(&material).to_hex().to_string()
}
