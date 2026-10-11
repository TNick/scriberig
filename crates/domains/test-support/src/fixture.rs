//! Defines the small contract implemented by domain conformance fixtures.

use scriberig_core::{EntityId, EntityRevisionRef, Provenance, RevisionId};

/// Describes optional behavior that a domain adapter supports.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CapabilitySet {
    /// Whether the fixture exposes an editable semantic projection.
    pub editable_projection: bool,
    /// Whether the fixture exposes derived analysis or knowledge.
    pub derived_analysis: bool,
}

/// The universal identity and revision returned by a domain inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Inspection {
    /// Universal entity identity.
    pub entity: EntityId,
    /// Exact immutable revision identity.
    pub revision: RevisionId,
}

/// Evidence that a projection edit crossed the shared change boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionEvidence {
    /// Whether the projection edit changed semantic content.
    pub semantic_change: bool,
    /// Whether reconciliation produced a shared change object.
    pub shared_change: bool,
    /// Whether the unchanged entity retained its identity and revision.
    pub unchanged_identity_reused: bool,
}

/// Evidence that derived data remains separate from canonical state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedEvidence {
    /// Exact canonical input used by the derived record.
    pub input: EntityRevisionRef,
    /// Recorded provenance for the derived record.
    pub provenance: Provenance,
    /// Whether discarding the derived record preserves canonical state.
    pub canonical_preserved: bool,
    /// Whether generated summaries remain classified as derived.
    pub generated_summary_is_derived: bool,
}

/// Supplies domain-specific cases to the generic conformance assertions.
pub trait ConformanceFixture {
    /// Returns the stable domain name used in assertion failures.
    ///
    /// # Returns
    /// Stable domain name.
    fn domain(&self) -> &'static str;

    /// Returns the canonical identity and revision exposed by inspection.
    ///
    /// # Returns
    /// Canonical identity and revision.
    fn canonical_inspection(&self) -> Inspection;

    /// Returns the identity and revision from the domain-specific inspection.
    ///
    /// # Returns
    /// Domain-specific identity and revision.
    fn domain_inspection(&self) -> Inspection;

    /// Returns the unchanged semantic case used to test revision reuse.
    ///
    /// # Returns
    /// Unchanged identity and revision.
    fn unchanged_inspection(&self) -> Inspection;

    /// Returns optional projection evidence.
    ///
    /// # Returns
    /// Projection evidence when editable projections are supported.
    fn projection(&self) -> Option<ProjectionEvidence> {
        None
    }

    /// Returns optional derived-data evidence.
    ///
    /// # Returns
    /// Derived-data evidence when analysis is supported.
    fn derived(&self) -> Option<DerivedEvidence> {
        None
    }

    /// Declares which optional cases this fixture supports.
    ///
    /// # Returns
    /// Capability flags for this fixture.
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::default()
    }
}
