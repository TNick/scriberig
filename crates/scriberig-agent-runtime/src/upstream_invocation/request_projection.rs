//! Captures safe `ScribeRig` provenance directly from upstream request values.

use crate::context::CompiledContext;
use rpi_ai::{Context, Model, ThinkingLevel};
use scriberig_session::{
    ModelContextRevisionRef, ModelRequestDetailMessage, ModelRequestDetailPart,
    ModelRequestDetailTool, ModelRequestMessage, ModelRequestProjection, ModelRequestTool,
    ModelRequestToolCall,
};

#[path = "request_projection/content.rs"]
mod content;
use content::{take_bounded, text_content, text_content_with_omissions};

#[path = "request_projection/fingerprint.rs"]
mod fingerprint;
use fingerprint::{
    RequestFingerprintInput, request_fingerprint_for, request_fingerprint_with_model_execution,
};

#[path = "request_projection/image_fingerprint.rs"]
mod image_fingerprint;
use image_fingerprint::image_fingerprints;

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
    /// Complete safe messages for on-demand detail reads.
    full: Vec<ModelRequestDetailMessage>,
    /// Whether non-text provider content was excluded from complete detail.
    content_complete: bool,
}

/// Holds bounded tool history and exact schema-fingerprint inputs.
struct ProjectedTools {
    /// Whether tool details were omitted because of the content budget.
    truncated: bool,
    /// Durable, bounded provider-visible tools.
    durable: Vec<ModelRequestTool>,
    /// Complete safe tools for on-demand detail reads.
    full: Vec<ModelRequestDetailTool>,
    /// Whether every safe tool-definition field was captured.
    content_complete: bool,
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
    let content_complete = messages.content_complete && tools.content_complete;
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
    let image_fingerprints = image_fingerprints(provider_context);
    let provider_profile = provider_profile.map(str::to_owned);
    let thinking_level = thinking_level.as_str().to_owned();
    let request_fingerprint = request_fingerprint_for(&RequestFingerprintInput {
        model,
        thinking_level: &thinking_level,
        messages: &messages.full,
        tools: &tools.full,
        image_fingerprints: &image_fingerprints,
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
        catalog_revision: None,
        upstream_runtime_revision: None,
        provider_response_id: None,
        model_execution_configuration: None,
        messages: messages.durable,
        full_messages: Some(messages.full),
        tools: tools.durable,
        full_tools: Some(tools.full),
        content_complete,
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
    if let Some(fingerprint) =
        request_fingerprint_with_model_execution(projection.request_fingerprint.as_deref(), model)
    {
        projection.request_fingerprint = Some(fingerprint);
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
    let mut full = Vec::new();
    let mut content_complete = true;
    if let Some(system_prompt) = &provider_input.system_prompt {
        append_message(
            "system",
            system_prompt,
            &mut budget,
            &mut truncated,
            &mut durable,
        );
        full.push(ModelRequestDetailMessage {
            role: "system".to_owned(),
            content: system_prompt.clone(),
            tool_name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
            parts: vec![ModelRequestDetailPart::Text(system_prompt.clone())],
        });
    }
    for message in &provider_input.messages {
        match message {
            rpi_ai::types::Message::User(message) => {
                let (content, omitted) = match &message.content {
                    rpi_ai::types::UserContent::Text(text) => (text.clone(), false),
                    rpi_ai::types::UserContent::Blocks(blocks) => (text_content(blocks), true),
                };
                if let rpi_ai::types::UserContent::Blocks(blocks) = &message.content {
                    content_complete &= !contains_image(blocks);
                }
                truncated |= omitted;
                let parts = match &message.content {
                    rpi_ai::types::UserContent::Text(text) => {
                        vec![ModelRequestDetailPart::Text(text.clone())]
                    }
                    rpi_ai::types::UserContent::Blocks(blocks) => safe_content_parts(blocks),
                };
                append_message("user", &content, &mut budget, &mut truncated, &mut durable);
                full.push(ModelRequestDetailMessage {
                    role: "user".to_owned(),
                    content: content.clone(),
                    tool_name: None,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    parts,
                });
            }
            rpi_ai::types::Message::Assistant(message) => {
                let (content, omitted) = text_content_with_omissions(&message.content);
                content_complete &= !contains_image(&message.content);
                truncated |= omitted;
                append_message(
                    "assistant",
                    &content,
                    &mut budget,
                    &mut truncated,
                    &mut durable,
                );
                let parts = safe_content_parts(&message.content);
                let tool_calls = parts
                    .iter()
                    .filter_map(|part| match part {
                        ModelRequestDetailPart::ToolCall(call) => Some(call.clone()),
                        ModelRequestDetailPart::Text(_) => None,
                    })
                    .collect();
                full.push(ModelRequestDetailMessage {
                    role: "assistant".to_owned(),
                    content,
                    tool_name: None,
                    tool_call_id: None,
                    tool_calls,
                    parts,
                });
            }
            rpi_ai::types::Message::ToolResult(message) => {
                let (content, omitted) = text_content_with_omissions(&message.content);
                content_complete &= !contains_image(&message.content);
                truncated |= omitted;
                append_message("tool", &content, &mut budget, &mut truncated, &mut durable);
                full.push(ModelRequestDetailMessage {
                    role: "tool".to_owned(),
                    content: content.clone(),
                    tool_name: Some(message.tool_name.clone()),
                    tool_call_id: Some(message.tool_call_id.clone()),
                    tool_calls: Vec::new(),
                    parts: safe_content_parts(&message.content),
                });
            }
        }
    }
    ProjectedMessages {
        budget,
        truncated,
        durable,
        full,
        content_complete,
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
fn project_tools(source: &[rpi_ai::types::Tool], mut budget: usize) -> ProjectedTools {
    let mut truncated = false;
    let mut durable = Vec::new();
    let mut full = Vec::new();
    let mut content_complete = true;
    for tool in source {
        let encoded_schema = serde_json::to_string(&tool.parameters).ok();
        content_complete &= encoded_schema.is_some();
        let constrained_sampling = serde_json::to_value(&tool.constrained_sampling);
        content_complete &= constrained_sampling.is_ok();
        full.push(ModelRequestDetailTool {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: encoded_schema.clone().unwrap_or_default(),
            constrained_sampling: constrained_sampling.ok().filter(|value| !value.is_null()),
        });
        if budget == 0 {
            truncated = true;
            continue;
        }
        let name = take_bounded(&tool.name, &mut budget);
        let description = take_bounded(&tool.description, &mut budget);
        truncated |= name.len() < tool.name.len() || description.len() < tool.description.len();
        let encoded_schema = encoded_schema.unwrap_or_default();
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
    }
    ProjectedTools {
        truncated,
        durable,
        full,
        content_complete,
    }
}

/// Appends provider-visible message text to bounded durable history.
///
/// # Parameters
/// - `role`: Upstream message role.
/// - `content`: Provider-visible text for this message.
/// - `budget`: Remaining safe history character budget.
/// - `truncated`: Whether any request content has been omitted.
/// - `projection`: Bounded durable messages being built.
fn append_message(
    role: &'static str,
    content: &str,
    budget: &mut usize,
    truncated: &mut bool,
    projection: &mut Vec<ModelRequestMessage>,
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
}

/// Checks whether a provider message contains image data excluded from detail.
///
/// # Parameters
/// - `content`: Provider message content blocks.
///
/// # Returns
/// Whether any image block was omitted from the safe text projection.
fn contains_image(content: &[rpi_ai::types::Content]) -> bool {
    content
        .iter()
        .any(|block| matches!(block, rpi_ai::types::Content::Image(_)))
}

/// Projects ordered provider-visible text and safe tool calls.
///
/// # Parameters
/// - `content`: Provider message content blocks.
///
/// # Returns
/// Ordered text and safe tool-call parts.
fn safe_content_parts(content: &[rpi_ai::types::Content]) -> Vec<ModelRequestDetailPart> {
    content
        .iter()
        .filter_map(|block| match block {
            rpi_ai::types::Content::Text(text) => {
                Some(ModelRequestDetailPart::Text(text.text.clone()))
            }
            rpi_ai::types::Content::ToolCall(call) => {
                Some(ModelRequestDetailPart::ToolCall(ModelRequestToolCall {
                    name: call.name.clone(),
                    arguments: serde_json::to_string(&call.arguments)
                        .expect("upstream tool-call arguments are JSON"),
                    id: Some(call.id.clone()),
                    namespace: call.namespace.clone(),
                }))
            }
            _ => None,
        })
        .collect()
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
