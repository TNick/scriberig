//! Renders complete safe model request provenance for semantic details.

/// Appends the safe request projection associated with one invocation.
///
/// - `output`: Markdown content receiving request details.
/// - `detail`: Bounded, credential-safe request projection.
pub(super) fn append_model_request(
    output: &mut String,
    detail: &scriberig_protocol::ModelRequestDetailResponse,
) {
    super::super::common::append_value(
        output,
        &format!(
            "Effective request · {} · {}",
            detail.invocation, detail.model
        ),
    );
    if let Some(provider) = &detail.provider_id {
        super::super::common::append_value(output, &format!("Provider: {provider}"));
    }
    if let Some(profile) = &detail.provider_profile {
        super::super::common::append_value(output, &format!("Provider profile: {profile}"));
    }
    if let Some(thinking) = &detail.thinking_level {
        super::super::common::append_value(output, &format!("Thinking level: {thinking}"));
    }
    if let Some(profile) = &detail.profile {
        super::super::common::append_value(
            output,
            &format!(
                "Profile revision: {} @ {}",
                profile.identity, profile.revision
            ),
        );
    }
    for component in detail.active_components.iter().take(64) {
        super::super::common::append_value(
            output,
            &format!(
                "Context component: {} @ {}",
                component.identity, component.revision
            ),
        );
    }
    if let Some(fingerprint) = &detail.context_fingerprint {
        super::super::common::append_value(output, &format!("Context fingerprint: {fingerprint}"));
    }
    if let Some(fingerprint) = &detail.request_fingerprint {
        super::super::common::append_value(output, &format!("Request fingerprint: {fingerprint}"));
    }
    if let Some(configuration) = &detail.model_execution_configuration {
        let summary = serde_json::to_string(configuration).unwrap_or_else(|_| "{}".to_owned());
        super::super::common::append_value(
            output,
            &format!("Upstream model execution configuration (partial): {summary}"),
        );
    }
    if detail.fragments.is_empty() {
        append_legacy_detail(output, detail);
    } else {
        append_fragments(output, &detail.fragments);
    }
    if let Some(report) = &detail.applied_settings {
        let summary = serde_json::to_string(report).unwrap_or_else(|_| "{}".to_owned());
        super::super::common::append_value(output, &format!("Applied settings: {summary}"));
    }
    if detail.truncated {
        super::super::common::append_value(output, "Effective request detail is truncated.");
    }
    if !detail.content_complete {
        super::super::common::append_value(
            output,
            "Some request content was unavailable in the safe capture.",
        );
    }
}

/// Appends every safe legacy field when a response has no paged fragments.
///
/// - `output`: Markdown content receiving request detail.
/// - `detail`: Provider-neutral request projection.
fn append_legacy_detail(
    output: &mut String,
    detail: &scriberig_protocol::ModelRequestDetailResponse,
) {
    for message in &detail.messages {
        super::super::common::append_complete_value(
            output,
            &format!("{}: {}", message.role, message.content),
        );
    }
    for tool in &detail.tools {
        super::super::common::append_complete_value(
            output,
            &format!("Tool: {} · {}", tool.name, tool.description),
        );
        if let Some(parameters) = &tool.parameters {
            super::super::common::append_complete_value(
                output,
                &format!("Tool parameters: {parameters}"),
            );
        }
    }
}

/// Appends ordered request fields after joining page-split fragments.
///
/// - `output`: Markdown content receiving complete request detail.
/// - `fragments`: Bounded fragments in stable request order.
fn append_fragments(
    output: &mut String,
    fragments: &[scriberig_protocol::ModelRequestDetailFragment],
) {
    let mut current: Option<scriberig_protocol::ModelRequestDetailFragment> = None;
    for fragment in fragments {
        if let Some(current) = current.as_mut().filter(|current| {
            current.kind == fragment.kind
                && current.index == fragment.index
                && current.parent_index == fragment.parent_index
                && current.field == fragment.field
                && current.role == fragment.role
                && current
                    .field_offset
                    .saturating_add(current.text.chars().count() as u64)
                    == fragment.field_offset
        }) {
            current.text.push_str(&fragment.text);
        } else {
            if let Some(previous) = current.take() {
                append_request_fragment(output, &previous);
            }
            current = Some(fragment.clone());
        }
    }
    if let Some(fragment) = current {
        append_request_fragment(output, &fragment);
    }
}

/// Appends one safe request field with its ScribeRig-specific label.
///
/// - `output`: Markdown destination receiving request provenance.
/// - `fragment`: Reassembled safe request field.
fn append_request_fragment(
    output: &mut String,
    fragment: &scriberig_protocol::ModelRequestDetailFragment,
) {
    let label = match (fragment.kind.as_str(), fragment.field.as_str()) {
        ("message" | "message_part", "content") => {
            fragment.role.as_deref().unwrap_or("message").to_owned()
        }
        ("message", "tool_name") => "Tool result name".to_owned(),
        ("message", "tool_call_id") => "Tool result call ID".to_owned(),
        ("tool_call", "name") => format!("Tool call {}", fragment.index),
        ("tool_call", "arguments") => format!("Tool call {} arguments", fragment.index),
        ("tool_call", "id") => format!("Tool call {} ID", fragment.index),
        ("tool_call", "namespace") => format!("Tool call {} namespace", fragment.index),
        ("tool", "name") => format!("Tool {} name", fragment.index),
        ("tool", "description") => format!("Tool {} description", fragment.index),
        ("tool", "parameters") => format!("Tool {} parameters", fragment.index),
        ("tool", "constrained_sampling") => format!("Tool {} sampling", fragment.index),
        _ => format!("{} {} {}", fragment.kind, fragment.index, fragment.field),
    };
    super::super::common::append_value(output, &label);
    super::super::common::append_complete_value(output, &fragment.text);
}

#[cfg(test)]
mod tests {
    use super::append_model_request;
    use scriberig_protocol::{ModelRequestDetailFragment, ModelRequestDetailResponse};

    /// Preserves complete safe request fragments longer than ordinary labels.
    #[test]
    fn expanded_prompt_keeps_the_complete_safe_effective_request() {
        let request_text = format!("{}tail-of-effective-request", "x".repeat(1200));
        let detail = ModelRequestDetailResponse {
            invocation: "ent_invocation".to_owned(),
            model: "provider/model".to_owned(),
            provider_id: Some("provider".to_owned()),
            provider_profile: None,
            thinking_level: None,
            profile: None,
            active_components: Vec::new(),
            context_fingerprint: None,
            request_fingerprint: None,
            catalog_revision: None,
            upstream_runtime_revision: None,
            provider_response_id: None,
            model_execution_configuration: None,
            messages: Vec::new(),
            tools: Vec::new(),
            applied_settings: None,
            applied_settings_fingerprint: None,
            truncated: false,
            content_complete: true,
            fragments: vec![ModelRequestDetailFragment {
                kind: "message".to_owned(),
                index: 0,
                parent_index: None,
                field: "content".to_owned(),
                field_offset: 0,
                role: Some("user".to_owned()),
                text: request_text,
            }],
            offset: Some(0),
            total_chars: Some(1225),
            next_offset: None,
        };
        let mut output = String::new();

        append_model_request(&mut output, &detail);

        assert!(output.contains("tail-of-effective-request"));
        assert!(!output.contains("Effective request detail is truncated."));
    }

    /// Preserves long and trailing legacy request messages without local caps.
    #[test]
    fn legacy_expanded_prompt_keeps_all_safe_messages() {
        let mut messages = (0..64)
            .map(|index| scriberig_protocol::ModelRequestMessageDetail {
                role: "user".to_owned(),
                content: format!("message {index}"),
            })
            .collect::<Vec<_>>();
        messages.push(scriberig_protocol::ModelRequestMessageDetail {
            role: "assistant".to_owned(),
            content: format!("{}legacy-tail", "x".repeat(1200)),
        });
        let detail = ModelRequestDetailResponse {
            invocation: "ent_invocation".to_owned(),
            model: "provider/model".to_owned(),
            provider_id: None,
            provider_profile: None,
            thinking_level: None,
            profile: None,
            active_components: Vec::new(),
            context_fingerprint: None,
            request_fingerprint: None,
            catalog_revision: None,
            upstream_runtime_revision: None,
            provider_response_id: None,
            model_execution_configuration: None,
            messages,
            tools: Vec::new(),
            applied_settings: None,
            applied_settings_fingerprint: None,
            truncated: false,
            content_complete: true,
            fragments: Vec::new(),
            offset: None,
            total_chars: None,
            next_offset: None,
        };
        let mut output = String::new();

        append_model_request(&mut output, &detail);

        assert!(output.contains("message 63"));
        assert!(output.contains("legacy-tail"));
        assert!(!output.contains("Effective request detail is truncated."));
    }
}
