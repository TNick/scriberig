//! Defines derived artifacts with inspectable exact dependencies.

use scriberig_core::{EntityRevisionRef, KnowledgeValue, Provenance};

/// Generic derived computation result retained outside canonical state.
#[derive(Clone, Debug, PartialEq)]
pub struct DerivedArtifact {
    /// Artifact or profile kind.
    pub kind: String,
    /// Exact immutable revisions used as inputs.
    pub dependencies: Vec<EntityRevisionRef>,
    /// Producer and configuration provenance.
    pub provenance: Provenance,
    /// Optional small inline result payload.
    pub result: Option<KnowledgeValue>,
    /// Processing status.
    pub status: DerivedStatus,
}

/// Lifecycle status of a derived artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DerivedStatus {
    /// Computation completed.
    Complete,
    /// Computation failed.
    Failed,
    /// Computation is pending.
    Pending,
}
