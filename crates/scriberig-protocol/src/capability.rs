//! Defines runtime capability discovery responses.

use crate::version::ProtocolVersion;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Describes one operation family available at runtime.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CapabilityInfo {
    /// Stable capability identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Whether this capability is currently usable.
    pub available: bool,
    /// Optional semantic domain scope.
    pub domain: Option<String>,
}

/// Names an optional runtime feature.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct RuntimeFeature {
    /// Stable feature identifier.
    pub id: String,
    /// Whether the feature is enabled.
    pub enabled: bool,
}

/// Returns protocol version and runtime capability metadata.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CapabilitiesResponse {
    /// Active protocol version.
    pub protocol: ProtocolVersion,
    /// Operation families exposed by this runtime.
    pub capabilities: Vec<CapabilityInfo>,
    /// Optional feature flags.
    pub features: Vec<RuntimeFeature>,
}
