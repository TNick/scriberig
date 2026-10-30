//! Builds bounded pages from safe model request detail.

use scriberig_session::{ModelRequestProjection, ModelRequestToolCall};

/// One bounded fragment from a safe provider-neutral request field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRequestDetailFragment {
    /// Fragment category such as `message`, `tool_call`, or `tool`.
    pub kind: String,
    /// Zero-based item index within the fragment category.
    pub index: u64,
    /// Parent message index for tool-call fragments.
    pub parent_index: Option<u64>,
    /// Field name represented by this fragment.
    pub field: String,
    /// Unicode-scalar offset within the field.
    pub field_offset: u64,
    /// Message role when the field belongs to a message.
    pub role: Option<String>,
    /// Safe text at this position in the field.
    pub text: String,
}

/// One bounded page of safe request detail for an invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRequestDetailPage {
    /// Safe content fragments included in this page.
    pub fragments: Vec<ModelRequestDetailFragment>,
    /// First Unicode-scalar offset represented by this page.
    pub offset: u64,
    /// Total Unicode-scalar length of the ordered safe detail stream.
    pub total_chars: u64,
    /// Offset to request next, absent at the end of the stream.
    pub next_offset: Option<u64>,
    /// Whether all safe text and tool content was captured.
    pub content_complete: bool,
}

/// One ordered field in the bounded request detail stream.
///
/// # Public members
/// None.
///
/// # Private members
/// - `kind`: Category used to identify the source item.
/// - `index`: Position within its category.
/// - `parent_index`: Parent message position for tool calls.
/// - `field`: Stable field label.
/// - `role`: Message role when the field belongs to a message.
/// - `text`: Safe text carried by this field.
#[derive(Clone, Debug)]
struct DetailField {
    /// Category used to identify the source item.
    kind: &'static str,
    /// Position within its category.
    index: u64,
    /// Parent message position for tool calls.
    parent_index: Option<u64>,
    /// Stable field label.
    field: &'static str,
    /// Message role when the field belongs to a message.
    role: Option<String>,
    /// Safe text carried by this field.
    text: String,
}

/// Projects one bounded page from a stored provider-neutral request.
///
/// The character budget applies across message text, tool calls, and tool
/// definitions. It can split a field so a single large value cannot create an
/// unbounded transport response.
///
/// # Parameters
/// - `request`: Durable safe request projection.
/// - `offset`: Requested Unicode-scalar offset in the ordered detail stream.
/// - `max_chars`: Requested page size, clamped to 1 through 32,768.
///
/// # Returns
/// One page of safe request text and its continuation offset.
#[must_use]
pub fn model_request_detail_page(
    request: &ModelRequestProjection,
    offset: u64,
    max_chars: u32,
) -> ModelRequestDetailPage {
    let fields = detail_fields(request);
    let total_chars = fields.iter().fold(0_u64, |total, field| {
        total.saturating_add(field.text.chars().count() as u64)
    });
    let start = offset.min(total_chars);
    let end = start
        .saturating_add(u64::from(max_chars.clamp(1, 32_768)))
        .min(total_chars);
    let mut field_start = 0_u64;
    let mut fragments = Vec::new();

    for field in fields {
        let field_end = field_start.saturating_add(field.text.chars().count() as u64);
        let fragment_start = start.max(field_start);
        let fragment_end = end.min(field_end);
        if fragment_start < fragment_end {
            if let (Ok(local_start), Ok(local_end)) = (
                usize::try_from(fragment_start - field_start),
                usize::try_from(fragment_end - field_start),
            ) {
                let chunk = field
                    .text
                    .chars()
                    .skip(local_start)
                    .take(local_end - local_start)
                    .collect();
                fragments.push(ModelRequestDetailFragment {
                    kind: field.kind.to_owned(),
                    index: field.index,
                    parent_index: field.parent_index,
                    field: field.field.to_owned(),
                    field_offset: fragment_start - field_start,
                    role: field.role,
                    text: chunk,
                });
            }
        }
        field_start = field_end;
    }

    ModelRequestDetailPage {
        fragments,
        offset: start,
        total_chars,
        next_offset: (end < total_chars).then_some(end),
        content_complete: request.content_complete,
    }
}

/// Lists safe request fields in their stable presentation order.
///
/// # Parameters
/// - `request`: Durable safe request projection.
///
/// # Returns
/// Ordered fields with their message, tool-call, or tool identity.
fn detail_fields(request: &ModelRequestProjection) -> Vec<DetailField> {
    let mut fields = Vec::new();
    append_message_fields(&mut fields, request);
    append_tool_fields(&mut fields, request);
    fields
}

/// Adds provider-visible message fields in request order.
///
/// # Parameters
/// - `fields`: Ordered safe request fields being collected.
/// - `request`: Durable safe request projection.
fn append_message_fields(fields: &mut Vec<DetailField>, request: &ModelRequestProjection) {
    let mut tool_call_index = 0_u64;
    if let Some(messages) = request.full_messages.as_ref() {
        for (index, message) in messages.iter().enumerate() {
            let message_index = index as u64;
            let role = Some(message.role.clone());
            if let Some(tool_name) = &message.tool_name {
                fields.push(DetailField {
                    kind: "message",
                    index: message_index,
                    parent_index: None,
                    field: "tool_name",
                    role: role.clone(),
                    text: tool_name.clone(),
                });
            }
            if let Some(tool_call_id) = &message.tool_call_id {
                fields.push(DetailField {
                    kind: "message",
                    index: message_index,
                    parent_index: None,
                    field: "tool_call_id",
                    role: role.clone(),
                    text: tool_call_id.clone(),
                });
            }
            if message.parts.is_empty() {
                fields.push(DetailField {
                    kind: "message",
                    index: message_index,
                    parent_index: None,
                    field: "content",
                    role: role.clone(),
                    text: message.content.clone(),
                });
                for call in &message.tool_calls {
                    append_tool_call_fields(
                        fields,
                        &mut tool_call_index,
                        message_index,
                        &message.role,
                        call,
                    );
                }
            } else {
                for (part_index, part) in message.parts.iter().enumerate() {
                    match part {
                        scriberig_session::ModelRequestDetailPart::Text(text) => {
                            fields.push(DetailField {
                                kind: "message_part",
                                index: part_index as u64,
                                parent_index: Some(message_index),
                                field: "content",
                                role: role.clone(),
                                text: text.clone(),
                            });
                        }
                        scriberig_session::ModelRequestDetailPart::ToolCall(call) => {
                            append_tool_call_fields(
                                fields,
                                &mut tool_call_index,
                                message_index,
                                &message.role,
                                call,
                            );
                        }
                    }
                }
            }
        }
    } else {
        for (index, message) in request.messages.iter().enumerate() {
            fields.push(DetailField {
                kind: "message",
                index: index as u64,
                parent_index: None,
                field: "content",
                role: Some(message.role.clone()),
                text: message.content.clone(),
            });
        }
    }
}

/// Adds safe tool schemas after all request messages.
///
/// # Parameters
/// - `fields`: Ordered safe request fields being collected.
/// - `request`: Durable safe request projection.
fn append_tool_fields(fields: &mut Vec<DetailField>, request: &ModelRequestProjection) {
    if let Some(tools) = request.full_tools.as_ref() {
        for (index, tool) in tools.iter().enumerate() {
            let index = index as u64;
            push_tool_field(fields, index, "name", tool.name.clone());
            push_tool_field(fields, index, "description", tool.description.clone());
            push_tool_field(fields, index, "parameters", tool.parameters.clone());
            if let Some(sampling) = &tool.constrained_sampling {
                push_tool_field(fields, index, "constrained_sampling", sampling.to_string());
            }
        }
    } else {
        for (index, tool) in request.tools.iter().enumerate() {
            let index = index as u64;
            push_tool_field(fields, index, "name", tool.name.clone());
            push_tool_field(fields, index, "description", tool.description.clone());
            if let Some(parameters) = &tool.parameters {
                push_tool_field(fields, index, "parameters", parameters.clone());
            }
        }
    }
}

/// Adds one safe field for a registered tool schema.
///
/// # Parameters
/// - `fields`: Ordered safe request fields being collected.
/// - `index`: Tool index in the request.
/// - `field`: Stable tool-schema field name.
/// - `text`: Safe tool-schema value.
fn push_tool_field(fields: &mut Vec<DetailField>, index: u64, field: &'static str, text: String) {
    fields.push(DetailField {
        kind: "tool",
        index,
        parent_index: None,
        field,
        role: None,
        text,
    });
}

/// Adds a normalized tool call's safe identity and arguments.
///
/// # Parameters
/// - `fields`: Ordered safe request fields being collected.
/// - `tool_call_index`: Mutable global tool-call index.
/// - `message_index`: Parent message index.
/// - `role`: Parent provider-neutral message role.
/// - `call`: Safe normalized tool-call data.
fn append_tool_call_fields(
    fields: &mut Vec<DetailField>,
    tool_call_index: &mut u64,
    message_index: u64,
    role: &str,
    call: &ModelRequestToolCall,
) {
    push_tool_call_field(
        fields,
        *tool_call_index,
        message_index,
        role,
        "name",
        call.name.clone(),
    );
    push_tool_call_field(
        fields,
        *tool_call_index,
        message_index,
        role,
        "arguments",
        call.arguments.clone(),
    );
    if let Some(id) = &call.id {
        push_tool_call_field(
            fields,
            *tool_call_index,
            message_index,
            role,
            "id",
            id.clone(),
        );
    }
    if let Some(namespace) = &call.namespace {
        push_tool_call_field(
            fields,
            *tool_call_index,
            message_index,
            role,
            "namespace",
            namespace.clone(),
        );
    }
    *tool_call_index = (*tool_call_index).saturating_add(1);
}

/// Adds one field belonging to a normalized tool call.
///
/// # Parameters
/// - `fields`: Ordered safe request fields being collected.
/// - `index`: Stable tool-call index.
/// - `message_index`: Parent message index.
/// - `role`: Parent provider-neutral message role.
/// - `field`: Stable tool-call field name.
/// - `text`: Safe field value.
fn push_tool_call_field(
    fields: &mut Vec<DetailField>,
    index: u64,
    message_index: u64,
    role: &str,
    field: &'static str,
    text: String,
) {
    fields.push(DetailField {
        kind: "tool_call",
        index,
        parent_index: Some(message_index),
        field,
        role: Some(role.to_owned()),
        text,
    });
}
