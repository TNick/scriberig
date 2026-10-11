//! Defines universal progressive-detail and information-cost hints.

/// Requests a bounded amount of semantic query detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum DetailLevel {
    /// Return only stable identity handles.
    Identity,
    /// Return orientation metadata and follow-up handles.
    Compact,
    /// Return semantic structure and supported summaries.
    Structure,
    /// Return all fields supported by the operation.
    Full,
}

/// Provides advisory coarse estimates for an operation result.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct InformationCost {
    /// Estimated number of returned semantic items.
    pub items: Option<u64>,
    /// Estimated encoded byte size.
    pub bytes: Option<u64>,
    /// Estimated model-token count.
    pub approx_tokens: Option<u64>,
}
