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
    /// Selected provider-neutral thinking level, when configured.
    pub thinking_level: Option<String>,
    /// Exact profile revision used to compile this request, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ModelContextRevisionReference>,
    /// Active context-component revisions in compilation order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_components: Vec<ModelContextRevisionReference>,
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
