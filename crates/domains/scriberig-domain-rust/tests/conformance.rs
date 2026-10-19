//! Checks Rust parsing and derived-analysis boundaries through the harness.

use scriberig_core::{EntityId, EntityRevisionRef, Provenance, ProvenanceSource, RevisionId};
use scriberig_domain_rust::parse;
use scriberig_domain_test_support::{
    CapabilitySet, ConformanceFixture, DerivedEvidence, Inspection, assert_conformance,
};

/// Supplies the representative Rust conformance case.
struct RustFixture;

impl ConformanceFixture for RustFixture {
    /// Returns the fixture domain name.
    ///
    /// Domain name.
    /// # Returns
    /// The operation result.
    fn domain(&self) -> &'static str {
        "rust"
    }
    /// Returns the canonical Rust identity.
    ///
    /// Canonical inspection.
    /// # Returns
    /// The operation result.
    fn canonical_inspection(&self) -> Inspection {
        Inspection {
            entity: EntityId::from_bytes([5; 16]),
            revision: RevisionId::new([6; 32]),
        }
    }
    /// Returns the Rust inspection.
    ///
    /// Domain inspection.
    /// # Returns
    /// The operation result.
    fn domain_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Returns the unchanged Rust identity.
    ///
    /// Unchanged inspection.
    /// # Returns
    /// The operation result.
    fn unchanged_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Declares Rust derived-analysis support.
    ///
    /// Derived capability flags.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            editable_projection: false,
            derived_analysis: true,
        }
    }
    /// Returns compiler provenance evidence.
    ///
    /// Compiler provenance evidence.
    /// # Returns
    /// The operation result.
    fn derived(&self) -> Option<DerivedEvidence> {
        let inspection = self.canonical_inspection();
        let input = EntityRevisionRef {
            entity_id: inspection.entity,
            revision_id: inspection.revision,
        };
        Some(DerivedEvidence {
            input,
            provenance: Provenance {
                source: ProvenanceSource::Compiler,
                producer: Some("rust-fixture".to_owned()),
                producer_version: Some("1".to_owned()),
                inputs: vec![input],
                profile: Some("syntax".to_owned()),
                note: None,
            },
            canonical_preserved: true,
            generated_summary_is_derived: true,
        })
    }
}

/// Verifies Rust syntax parsing and derived provenance boundaries.
#[test]
fn rust_fixture_is_universal() {
    assert!(parse("fn fixture() {}").is_ok());
    assert_conformance(&RustFixture);
}
