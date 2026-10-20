//! Captures bounded safe request data before provider-specific serialization.

use crate::provider::ModelRequest;

const MAX_CAPTURED_CONTENT_CHARS: usize = 15_000;

/// Projects provider-neutral request fields without transport metadata.
///
/// - `request`: Normalized request immediately before adapter execution.
/// - `provider_id`: Resolved public provider identity, when available.
/// - `provider_profile`: Safe `ScribeRig` provider profile, when available.
/// - `context`: Compiled `ScribeRig` context bound to this invocation.
///
/// A bounded request projection containing messages and trusted tool schemas.
/// # Returns
/// The operation result.
pub(super) fn capture(
    request: &ModelRequest,
    provider_id: Option<String>,
    provider_profile: Option<String>,
    context: Option<&crate::context::CompiledContext>,
) -> scriberig_session::ModelRequestProjection {
    let mut budget = MAX_CAPTURED_CONTENT_CHARS;
    let mut truncated = false;
    let mut messages = Vec::new();
    for message in &request.messages {
        if budget == 0 {
            truncated = true;
            break;
        }
        let captured_content = take_bounded(&message.content, &mut budget);
        truncated |= captured_content.chars().count() < message.content.chars().count();
        let role = match message.role {
            crate::provider::MessageRole::System => "system",
            crate::provider::MessageRole::User => "user",
            crate::provider::MessageRole::Assistant => "assistant",
            crate::provider::MessageRole::Tool => "tool",
        };
        messages.push(scriberig_session::ModelRequestMessage {
            role: role.to_owned(),
            content: captured_content,
        });
    }
    let mut tools = Vec::new();
    for tool in &request.tools {
        if budget == 0 {
            truncated = true;
            break;
        }
        let name = take_bounded(&tool.name, &mut budget);
        let description = take_bounded(&tool.description, &mut budget);
        truncated |= name.len() < tool.name.len() || description.len() < tool.description.len();
        let encoded_schema = serde_json::to_string(&tool.parameters).unwrap_or_default();
        let parameters = if encoded_schema.chars().count() <= budget {
            budget -= encoded_schema.chars().count();
            Some(encoded_schema)
        } else {
            truncated = true;
            None
        };
        tools.push(scriberig_session::ModelRequestTool {
            name,
            description,
            parameters,
        });
    }
    let profile = context.map(|context| scriberig_session::ModelContextRevisionRef {
        identity: context.profile.identity.clone(),
        revision: context.profile.revision.clone(),
    });
    let active_components = context
        .into_iter()
        .flat_map(|context| context.active_components.iter())
        .map(|reference| scriberig_session::ModelContextRevisionRef {
            identity: reference.identity.clone(),
            revision: reference.revision.clone(),
        })
        .collect::<Vec<_>>();
    let context_fingerprint = context.map(context_fingerprint_for);
    let request_fingerprint = request_fingerprint_for(
        request,
        provider_id.as_deref(),
        provider_profile.as_deref(),
        profile.as_ref(),
        &active_components,
        context_fingerprint.as_deref(),
    );
    scriberig_session::ModelRequestProjection {
        ordinal: 0,
        model: request.model.clone(),
        provider_id,
        provider_profile,
        thinking_level: request.thinking_level.clone(),
        profile,
        active_components,
        context_fingerprint,
        request_fingerprint: Some(request_fingerprint),
        messages,
        tools,
        truncated,
    }
}

/// Fingerprints the exact compiled `ScribeRig` context used by a request.
///
/// - `context`: Compiled context and immutable profile/component references.
///
/// Stable BLAKE3 context fingerprint.
///
/// # Returns
/// The hex-encoded context fingerprint.
fn context_fingerprint_for(context: &crate::context::CompiledContext) -> String {
    let material = serde_json::to_vec(&(
        "scriberig-context-v1",
        (&context.profile.identity, &context.profile.revision),
        context
            .active_components
            .iter()
            .map(|reference| (&reference.identity, &reference.revision))
            .collect::<Vec<_>>(),
        &context.canonical,
    ))
    .expect("context fingerprint inputs are serializable");
    blake3::hash(&material).to_hex().to_string()
}

/// Fingerprints the normalized request and `ScribeRig` context references.
///
/// # Parameters
/// - `request`: Normalized request immediately before adapter execution.
/// - `provider_id`: Resolved public provider identity, when available.
/// - `provider_profile`: Safe provider profile provenance, when available.
/// - `profile`: Exact profile revision reference, when present.
/// - `active_components`: Exact active component revision references.
/// - `context_fingerprint`: Fingerprint of the compiled context, when present.
///
/// Stable BLAKE3 request fingerprint.
///
/// # Returns
/// The hex-encoded request fingerprint.
fn request_fingerprint_for(
    request: &ModelRequest,
    provider_id: Option<&str>,
    provider_profile: Option<&str>,
    profile: Option<&scriberig_session::ModelContextRevisionRef>,
    active_components: &[scriberig_session::ModelContextRevisionRef],
    context_fingerprint: Option<&str>,
) -> String {
    let context_identity = context_fingerprint;
    let messages = request
        .messages
        .iter()
        .map(|message| {
            let role = match message.role {
                crate::provider::MessageRole::System => "system",
                crate::provider::MessageRole::User => "user",
                crate::provider::MessageRole::Assistant => "assistant",
                crate::provider::MessageRole::Tool => "tool",
            };
            (
                role,
                message.tool_call_id.as_deref(),
                message.content.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let tools = request
        .tools
        .iter()
        .map(|tool| (&tool.name, &tool.description, &tool.parameters))
        .collect::<Vec<_>>();
    let tool_choice = match &request.tool_choice {
        crate::provider::ToolChoice::Auto => ("auto", None),
        crate::provider::ToolChoice::None => ("none", None),
        crate::provider::ToolChoice::Required(name) => ("required", Some(name.as_str())),
    };
    let profile_identity = profile.map(|reference| (&reference.identity, &reference.revision));
    let component_identity = active_components
        .iter()
        .map(|reference| (&reference.identity, &reference.revision))
        .collect::<Vec<_>>();
    let material = serde_json::to_vec(&(
        "scriberig-request-v1",
        provider_id,
        provider_profile,
        &request.model,
        request.thinking_level.as_deref(),
        tool_choice,
        &request.response_schema,
        request.temperature.map(f32::to_bits),
        request.max_tokens,
        profile_identity,
        component_identity,
        context_identity,
        messages,
        tools,
    ))
    .expect("request fingerprint inputs are serializable");
    blake3::hash(&material).to_hex().to_string()
}

/// Copies at most the remaining character budget from one string.
///
/// - `value`: Content to copy.
/// - `budget`: Mutable remaining character allowance.
///
/// The bounded prefix and updated remaining budget.
/// # Returns
/// The operation result.
fn take_bounded(value: &str, budget: &mut usize) -> String {
    let bounded = value.chars().take(*budget).collect::<String>();
    *budget -= bounded.chars().count();
    bounded
}

#[cfg(test)]
#[path = "request_projection/tests.rs"]
mod tests;
