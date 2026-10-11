//! Defines protocol version negotiation values.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Identifies a protocol major and additive minor version.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProtocolVersion {
    /// Incompatible wire contract version.
    pub major: u16,
    /// Additive wire contract version.
    pub minor: u16,
}

impl Default for ProtocolVersion {
    /// Returns the initial protocol major version.
    ///
    /// # Returns
    /// The initial major version one and minor version zero.
    fn default() -> Self {
        Self { major: 1, minor: 0 }
    }
}

/// Exchanges supported and active protocol versions.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct VersionExchange {
    /// Version selected for this connection.
    pub active: ProtocolVersion,
    /// Versions accepted by the peer.
    pub supported: Vec<ProtocolVersion>,
}
