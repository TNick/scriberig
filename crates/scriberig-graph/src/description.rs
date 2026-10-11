//! Defines derived descriptions bound to exact source revisions.

use scriberig_core::{EntityRevisionRef, Provenance};

/// Optional information-cost estimates for a description.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InformationCost {
    /// Byte length when known.
    pub bytes: Option<u64>,
    /// Character count when known.
    pub characters: Option<u64>,
    /// Estimated token count when known.
    pub estimated_tokens: Option<u64>,
}

/// A derived textual description of one exact entity revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Description {
    /// Exact source entity and revision.
    pub source: EntityRevisionRef,
    /// Description profile identity.
    pub profile: String,
    /// Derived text.
    pub text: String,
    /// Origin and exact inputs used to produce the text.
    pub provenance: Provenance,
    /// Optional size estimates.
    pub information_cost: Option<InformationCost>,
}
