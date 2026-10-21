//! Captures safe `ScribeRig` provenance directly from upstream request values.

use crate::context::CompiledContext;
use rpi_ai::{Context, Model, ThinkingLevel};
use scriberig_session::{
    ModelContextRevisionRef, ModelRequestMessage, ModelRequestProjection, ModelRequestTool,
};

#[path = "request_projection/content.rs"]
mod content;
use content::{take_bounded, text_content, text_content_with_omissions};

#[cfg(test)]
#[path = "request_projection/tests.rs"]
mod tests;

const MAX_CAPTURED_CONTENT_CHARS: usize = 15_000;

/// Holds bounded message history and exact request-fingerprint inputs.
struct ProjectedMessages {
    /// Remaining character allowance after message projection.
    budget: usize,
    /// Whether non-text data or content limits omitted request material.
    truncated: bool,
    /// Durable, bounded provider-visible messages.
    durable: Vec<ModelRequestMessage>,
    /// Full in-memory message values used for request identity.
    fingerprint: Vec<(&'static str, Option<String>, String)>,
}

/// Holds bounded tool history and exact schema-fingerprint inputs.
struct ProjectedTools<'a> {
    /// Whether tool details were omitted because of the content budget.
    truncated: bool,
    /// Durable, bounded provider-visible tools.
    durable: Vec<ModelRequestTool>,
    /// Source tool names, descriptions, and schemas for request identity.
    fingerprint: Vec<(&'a String, &'a String, &'a rpi_ai::types::Schema)>,
}

/// Captures a bounded request projection before provider serialization.
///
/// # Parameters
/// - `model`: Exact model bound to this branch invocation.
/// - `provider_context`: Provider-neutral context created by `rpi-agent`.
/// - `provider_profile`: Safe `ScribeRig` provider profile, when selected.
/// - `compiled_context`: Exact `ScribeRig` context compilation.
/// - `thinking_level`: Selected upstream thinking level.
///
/// # Returns
/// A bounded `ScribeRig` history projection without transport secrets.
pub(super) fn capture_provider_request(
    model: &Model,
    provider_context: &Context,
    provider_profile: Option<&str>,
    compiled_context: &CompiledContext,
    thinking_level: ThinkingLevel,
) -> ModelRequestProjection {
    let messages = project_messages(provider_context);
    let tools = project_tools(&provider_context.tools, messages.budget);
    let truncated = messages.truncated || tools.truncated;
    let profile = Some(ModelContextRevisionRef {
        identity: compiled_context.profile.identity.clone(),
        revision: compiled_context.profile.revision.clone(),
    });
    let active_components = compiled_context
        .active_components
        .iter()
        .map(|reference| ModelContextRevisionRef {
            identity: reference.identity.clone(),
            revision: reference.revision.clone(),
        })
        .collect::<Vec<_>>();
    let context_fingerprint = Some(context_fingerprint_for(compiled_context));
    let provider_profile = provider_profile.map(str::to_owned);
    let thinking_level = thinking_level.as_str().to_owned();
    let request_fingerprint = request_fingerprint_for(&RequestFingerprintInput {
        model,
        thinking_level: &thinking_level,
        messages: &messages.fingerprint,
        tools: &tools.fingerprint,
        provider_profile: provider_profile.as_deref(),
        profile: profile.as_ref(),
        active_components: &active_components,
        context_fingerprint: context_fingerprint.as_deref(),
    });
    let mut projection = ModelRequestProjection {
        ordinal: 0,
        model: model.id.clone(),
        provider_id: Some(model.provider.clone()),
        provider_profile,
        thinking_level: Some(thinking_level),
        profile,
        active_components,
        context_fingerprint,
        request_fingerprint: Some(request_fingerprint),
        model_execution_configuration: None,
        messages: messages.durable,
        tools: tools.durable,
        applied_settings: None,
        applied_settings_fingerprint: None,
        truncated,
    };
    let execution_configuration = rpi_model_runtime::safe_model_execution_configuration(model);
    if let Ok(configuration) = serde_json::to_value(execution_configuration) {
        let material = serde_json::to_vec(&(
            "scriberig-request-with-model-configuration-v1",
            projection.request_fingerprint.as_deref(),
            &configuration,
        ))
        .expect("safe upstream model configuration is serializable");
        projection.request_fingerprint = Some(blake3::hash(&material).to_hex().to_string());
        projection.model_execution_configuration = Some(configuration);
    }
    projection
}

/// Projects provider-visible messages into bounded history and fingerprint data.
///
/// # Parameters
/// - `provider_input`: Exact upstream context prepared for the provider.
///
/// # Returns
/// Bounded durable messages, complete fingerprint inputs, and remaining budget.
fn project_messages(provider_input: &Context) -> ProjectedMessages {
    let mut budget = MAX_CAPTURED_CONTENT_CHARS;
    let mut truncated = false;
    let mut durable = Vec::new();
    let mut fingerprint = Vec::new();
    if let Some(system_prompt) = &provider_input.system_prompt {
        append_message(
            "system",
            None,
            system_prompt,
            &mut budget,
            &mut truncated,
            &mut durable,
            &mut fingerprint,
        );
    }
    for message in &provider_input.messages {
        match message {
            rpi_ai::types::Message::User(message) => {
                let (content, omitted) = match &message.content {
                    rpi_ai::types::UserContent::Text(text) => (text.clone(), false),
                    rpi_ai::types::UserContent::Blocks(blocks) => (text_content(blocks), true),
                };
                truncated |= omitted;
                append_message(
                    "user",
                    None,
                    &content,
                    &mut budget,
                    &mut truncated,
                    &mut durable,
                    &mut fingerprint,
                );
            }
            rpi_ai::types::Message::Assistant(message) => {
                let (content, omitted) = text_content_with_omissions(&message.content);
                truncated |= omitted;
                append_message(
                    "assistant",
                    None,
                    &content,
                    &mut budget,
                    &mut truncated,
                    &mut durable,
                    &mut fingerprint,
                );
            }
            rpi_ai::types::Message::ToolResult(message) => {
                let (content, omitted) = text_content_with_omissions(&message.content);
                truncated |= omitted;
                append_message(
                    "tool",
                    Some(&message.tool_call_id),
                    &content,
                    &mut budget,
                    &mut truncated,
                    &mut durable,
                    &mut fingerprint,
                );
            }
        }
    }
    ProjectedMessages {
        budget,
        truncated,
        durable,
        fingerprint,
    }
}

/// Projects upstream tool schemas within the remaining provenance budget.
///
/// # Parameters
/// - `source`: Provider-visible upstream tools.
/// - `budget`: Remaining character budget after message projection.
///
/// # Returns
/// Bounded durable tool descriptions and exact in-memory fingerprint inputs.
fn project_tools(source: &[rpi_ai::types::Tool], mut budget: usize) -> ProjectedTools<'_> {
    let mut truncated = false;
    let mut durable = Vec::new();
    let mut fingerprint = Vec::new();
    for tool in source {
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
            Some(encoded_schema.clone())
        } else {
            truncated = true;
            None
        };
        durable.push(ModelRequestTool {
            name,
            description,
            parameters,
        });
        fingerprint.push((&tool.name, &tool.description, &tool.parameters));
    }
    ProjectedTools {
        truncated,
        durable,
        fingerprint,
    }
}

/// Appends a message to the bounded history and full request fingerprint.
///
/// # Parameters
/// - `role`: Upstream message role.
/// - `tool_call_id`: Tool call identity when the message is a tool result.
/// - `content`: Provider-visible text for this message.
/// - `budget`: Remaining safe history character budget.
/// - `truncated`: Whether any request content has been omitted.
/// - `projection`: Bounded durable messages being built.
/// - `fingerprint`: Complete in-memory message inputs for request identity.
fn append_message(
    role: &'static str,
    tool_call_id: Option<&str>,
    content: &str,
    budget: &mut usize,
    truncated: &mut bool,
    projection: &mut Vec<ModelRequestMessage>,
    fingerprint: &mut Vec<(&'static str, Option<String>, String)>,
) {
    if *budget == 0 {
        *truncated = true;
    } else {
        let captured = take_bounded(content, budget);
        *truncated |= captured.chars().count() < content.chars().count();
        projection.push(ModelRequestMessage {
            role: role.to_owned(),
            content: captured,
        });
    }
    fingerprint.push((role, tool_call_id.map(str::to_owned), content.to_owned()));
}

/// Fingerprints the exact compiled `ScribeRig` context.
///
/// # Parameters
/// - `context`: Compiled context and immutable profile/component references.
///
/// # Returns
/// The hex-encoded context fingerprint.
fn context_fingerprint_for(context: &CompiledContext) -> String {
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

/// Fingerprints upstream request content with `ScribeRig` context identity.
///
/// # Parameters
/// - `model`: Exact upstream model selected for the invocation.
/// - `thinking_level`: Upstream thinking level selected for the invocation.
/// - `messages`: Complete provider-visible message inputs.
/// - `tools`: Complete upstream tool schemas supplied to the provider.
/// - `provider_profile`: Safe `ScribeRig` provider profile, when selected.
/// - `profile`: Exact `ScribeRig` profile revision reference.
/// - `active_components`: Exact active context component references.
/// - `context_fingerprint`: Fingerprint of the compiled `ScribeRig` context.
///
/// # Returns
/// The hex-encoded request fingerprint.
/// Borrows the inputs that define exact request and semantic context identity.
/// Borrows the inputs that define exact request and semantic context identity.
///
/// Fields hold the canonical model, bounded message inputs, complete tool
/// schemas, and immutable `ScribeRig` context references for one fingerprint.
struct RequestFingerprintInput<'a> {
    /// Exact upstream model selected for this invocation.
    model: &'a Model,
    /// Upstream thinking level selected for this invocation.
    thinking_level: &'a str,
    /// Provider-visible messages supplied to the selected provider.
    messages: &'a [(&'a str, Option<String>, String)],
    /// Provider-visible tools supplied to the selected provider.
    tools: &'a [(&'a String, &'a String, &'a rpi_ai::types::Schema)],
    /// Safe `ScribeRig` provider profile identifier, if selected.
    provider_profile: Option<&'a str>,
    /// Exact immutable `ScribeRig` profile revision.
    profile: Option<&'a ModelContextRevisionRef>,
    /// Exact immutable references for active context components.
    active_components: &'a [ModelContextRevisionRef],
    /// Fingerprint of the compiled `ScribeRig` context.
    context_fingerprint: Option<&'a str>,
}

/// Fingerprints the safe normalized request inputs and semantic context.
///
/// This pre-provider digest is extended with the safe model-configuration
/// snapshot by `capture_provider_request` and may be extended with an upstream
/// applied-settings report after provider preparation. Omitted secrets,
/// opaque properties, and other fields mean it is not a complete provider
/// request-equivalence fingerprint.
///
/// # Parameters
/// - `input`: Exact model, request content, `ScribeRig` context, and profile data.
///
/// # Returns
/// The hex-encoded base request fingerprint.
fn request_fingerprint_for(input: &RequestFingerprintInput<'_>) -> String {
    let profile_identity = input
        .profile
        .map(|reference| (&reference.identity, &reference.revision));
    let component_identity = input
        .active_components
        .iter()
        .map(|reference| (&reference.identity, &reference.revision))
        .collect::<Vec<_>>();
    let material = serde_json::to_vec(&(
        "scriberig-request-v1",
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
    ))
    .expect("request fingerprint inputs are serializable");
    blake3::hash(&material).to_hex().to_string()
}
