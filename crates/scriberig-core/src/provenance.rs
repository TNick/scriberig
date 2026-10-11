//! Defines origin and exact-input provenance for semantic knowledge.

use crate::{EntityId, RevisionId};

/// Identifies one exact immutable entity revision used as provenance input.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntityRevisionRef {
    /// Stable entity identity.
    pub entity_id: EntityId,
    /// Exact immutable revision identity.
    pub revision_id: RevisionId,
}

/// Identifies the kind of source that produced a value or artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProvenanceSource {
    /// A human user authored or asserted the value.
    User,
    /// An authoritative external import produced the value.
    AuthoritativeImport,
    /// A parser produced the value.
    Parser,
    /// A compiler produced the value.
    Compiler,
    /// Static analysis produced the value.
    StaticAnalysis,
    /// An agent produced the value.
    Agent,
    /// A model produced the value.
    ModelGenerated,
    /// A system component produced the value.
    System,
    /// An external tool produced the value.
    ExternalTool,
    /// A migration produced the value.
    Migration,
    /// An extensible producer category.
    Custom(String),
}

/// Records origin and exact immutable dependencies without implying trust.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provenance {
    /// Source category that produced the knowledge.
    pub source: ProvenanceSource,
    /// Optional producer, tool, or model identifier.
    pub producer: Option<String>,
    /// Optional producer implementation version.
    pub producer_version: Option<String>,
    /// Exact immutable entity revisions used as inputs.
    pub inputs: Vec<EntityRevisionRef>,
    /// Optional profile or configuration identifier.
    pub profile: Option<String>,
    /// Optional short explanation of the derivation or assertion.
    pub note: Option<String>,
}
