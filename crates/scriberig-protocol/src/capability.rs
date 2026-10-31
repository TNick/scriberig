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

/// Identifies one external CAD format whose provider can be optionally installed.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CadProviderFormat {
    /// Native Drawing format.
    Dwg,
    /// Drawing Exchange Format.
    Dxf,
    /// Provider-defined external CAD format.
    Other(String),
}

/// Reports how an optional CAD format provider is available at this runtime.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CadProviderAvailability {
    /// A provider is initialized and its capabilities are included in this record.
    Available,
    /// No provider is configured for this format.
    Unavailable,
    /// A configured provider failed to initialize.
    InitializationFailed,
}

/// Reports provider-neutral availability, operations, and fidelity for one CAD format.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CadProviderCapabilityInfo {
    /// External CAD format covered by this record.
    pub format: CadProviderFormat,
    /// Current provider state.
    pub availability: CadProviderAvailability,
    /// Safe provider identifier, present only when initialization succeeded.
    pub provider_id: Option<String>,
    /// Safe provider version, present only when initialization succeeded.
    pub provider_version: Option<String>,
    /// Provider can read this format into shared CAD semantics.
    pub can_read: bool,
    /// Provider can render supported semantic candidates to this format.
    pub can_write: bool,
    /// Provider can reopen and validate rendered candidates.
    pub can_validate: bool,
    /// Preservation guarantee for content outside shared CAD semantics.
    pub opaque_preservation: CadOpaquePreservation,
}

/// States the provider guarantee for source content outside shared CAD semantics.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CadOpaquePreservation {
    /// Provider guarantees unrelated source content survives semantic edits.
    Preserved,
    /// Preservation has not been established.
    Unproven,
    /// Provider is known to discard unrelated source content.
    NotPreserved,
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
    /// Optional native CAD provider state for this server, independent of client installation.
    #[serde(default, rename = "cadProviders", alias = "cad_providers")]
    pub cad_providers: Vec<CadProviderCapabilityInfo>,
}
