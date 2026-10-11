//! Checks PDF source authority and derived extraction through the harness.

use scriberig_core::{EntityId, EntityRevisionRef, Provenance, ProvenanceSource, RevisionId};
use scriberig_domain_pdf::{PdfDocument, extract, import_source};
use scriberig_domain_test_support::{
    CapabilitySet, ConformanceFixture, DerivedEvidence, Inspection, assert_conformance,
};

/// Supplies the representative PDF conformance case.
struct PdfFixture;

impl ConformanceFixture for PdfFixture {
    /// Returns the fixture domain name.
    ///
    /// # Returns
    /// Domain name.
    fn domain(&self) -> &'static str {
        "pdf"
    }
    /// Returns the canonical PDF identity.
    ///
    /// # Returns
    /// Canonical inspection.
    fn canonical_inspection(&self) -> Inspection {
        Inspection {
            entity: EntityId::new([7; 16]),
            revision: RevisionId::new([8; 32]),
        }
    }
    /// Returns the PDF inspection.
    ///
    /// # Returns
    /// Domain inspection.
    fn domain_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Returns the unchanged PDF identity.
    ///
    /// # Returns
    /// Unchanged inspection.
    fn unchanged_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Declares PDF derived extraction support.
    ///
    /// # Returns
    /// Derived capability flags.
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            editable_projection: false,
            derived_analysis: true,
        }
    }
    /// Returns extraction provenance evidence.
    ///
    /// # Returns
    /// Extraction provenance evidence.
    fn derived(&self) -> Option<DerivedEvidence> {
        let inspection = self.canonical_inspection();
        let input = EntityRevisionRef {
            entity_id: inspection.entity,
            revision_id: inspection.revision,
        };
        Some(DerivedEvidence {
            input,
            provenance: Provenance {
                source: ProvenanceSource::Parser,
                producer: Some("pdf-fixture".to_owned()),
                producer_version: Some("1".to_owned()),
                inputs: vec![input],
                profile: Some("text".to_owned()),
                note: None,
            },
            canonical_preserved: true,
            generated_summary_is_derived: true,
        })
    }
}

/// Verifies exact PDF bytes remain authoritative while extraction is derived.
#[test]
fn pdf_fixture_is_universal() {
    let source = import_source(b"%PDF-1.7 (fixture)".to_vec()).expect("source");
    let document = extract(&PdfDocument::from_source(source), "fixture", "1");
    assert_eq!(document.source.bytes, b"%PDF-1.7 (fixture)");
    assert_conformance(&PdfFixture);
}
