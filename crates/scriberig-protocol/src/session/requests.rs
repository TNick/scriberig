//! Defines requests values for durable session transport.

use crate::SessionReference;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests one durable session tree.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionReadRequest {
    /// Session identity to inspect.
    pub session: SessionReference,
    /// Optional branch whose activity should be loaded on demand.
    #[serde(default)]
    pub branch: Option<crate::BranchReference>,
}

/// Requests one stored model request projection by durable invocation identity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestDetailRequest {
    /// Session containing the invocation.
    pub session: SessionReference,
    /// Branch containing the invocation.
    pub branch: crate::BranchReference,
    /// Stable model invocation activity identity.
    pub invocation: String,
    /// Unicode-scalar offset into the ordered safe detail text stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Maximum Unicode scalar values returned in this page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_chars: Option<u32>,
}

/// One bounded fragment from a safe model request detail page.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestDetailFragment {
    /// Whether the fragment belongs to a message, tool call, or tool.
    pub kind: String,
    /// Zero-based index within the selected fragment kind.
    pub index: u64,
    /// Parent message index for a tool-call fragment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_index: Option<u64>,
    /// Text field represented by this fragment.
    pub field: String,
    /// Unicode-scalar offset within the field.
    pub field_offset: u64,
    /// Message role when this is a message fragment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Safe text at this position in the field.
    pub text: String,
}

/// One safe message in a stored provider-neutral model request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestMessageDetail {
    /// Provider-neutral message role.
    pub role: String,
    /// Captured message content.
    pub content: String,
}

/// One trusted tool definition in a stored model request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestToolDetail {
    /// Stable provider-facing tool name.
    pub name: String,
    /// Safe descriptive text.
    pub description: String,
    /// Bounded JSON Schema text when it fit in the capture budget.
    pub parameters: Option<String>,
}

/// Safe exact reference to one profile or context-component revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelContextRevisionReference {
    /// Stable semantic identity.
    pub identity: String,
    /// Exact immutable revision identity.
    pub revision: String,
}

/// Safe provider-neutral detail captured for one model invocation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestDetailResponse {
    /// Stable model invocation identity.
    pub invocation: String,
    /// Normalized model identifier.
    pub model: String,
    /// Resolved upstream provider identity, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// Safe `ScribeRig` provider profile identifier, when selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_profile: Option<String>,
    /// Selected provider-neutral thinking level, when configured.
    pub thinking_level: Option<String>,
    /// Exact profile revision used to compile this request, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ModelContextRevisionReference>,
    /// Active context-component revisions in compilation order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_components: Vec<ModelContextRevisionReference>,
    /// Secret-free identity of the compiled `ScribeRig` context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_fingerprint: Option<String>,
    /// Secret-free fingerprint of captured request provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_fingerprint: Option<String>,
    /// Upstream-defined partial safe model execution configuration.
    ///
    /// Coverage metadata must be retained; this is not a complete wire-request
    /// equivalence claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_execution_configuration: Option<serde_json::Value>,
    /// Ordered normalized messages.
    pub messages: Vec<ModelRequestMessageDetail>,
    /// Trusted tools exposed to the model.
    pub tools: Vec<ModelRequestToolDetail>,
    /// Opaque safe applied-settings report supplied by the upstream provider.
    ///
    /// The report schema is owned by upstream and remains uninterpreted here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_settings: Option<serde_json::Value>,
    /// Versioned digest of the applied-settings report, when captured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_settings_fingerprint: Option<String>,
    /// Whether content was omitted to respect capture bounds.
    pub truncated: bool,
    /// Whether all safe text and tool content was captured.
    #[serde(default)]
    pub content_complete: bool,
    /// Fragments included in the requested page; empty for legacy reads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fragments: Vec<ModelRequestDetailFragment>,
    /// First Unicode-scalar offset represented by this page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Total Unicode-scalar length of the ordered safe detail text stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_chars: Option<u64>,
    /// Offset to request next, absent when this page reaches the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u64>,
}

/// Requests expanded bounded detail for one durable tool result.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ToolResultDetailRequest {
    /// Session containing the result.
    pub session: SessionReference,
    /// Branch containing the result.
    pub branch: crate::BranchReference,
    /// Stable tool result identity.
    pub result: String,
}

/// Safe bounded structured detail for one durable tool result.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ToolResultDetailResponse {
    /// Stable tool result identity.
    pub result: String,
    /// Whitelisted provider-neutral result fields.
    pub detail: serde_json::Value,
    /// Whether safe fields were truncated to satisfy the response bound.
    pub truncated: bool,
}
