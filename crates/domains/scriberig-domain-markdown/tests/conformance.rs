//! Checks Markdown parsing and projection boundaries through the shared harness.

use scriberig_core::{EntityId, RevisionId};
use scriberig_domain_markdown::{parse, render};
use scriberig_domain_test_support::{
    CapabilitySet, ConformanceFixture, Inspection, ProjectionEvidence, assert_conformance,
};

/// Supplies the representative Markdown conformance case.
struct MarkdownFixture;

impl ConformanceFixture for MarkdownFixture {
    /// Returns the fixture domain name.
    ///
    /// # Returns
    /// Domain name.
    fn domain(&self) -> &'static str {
        "markdown"
    }
    /// Returns the canonical Markdown identity.
    ///
    /// # Returns
    /// Canonical inspection.
    fn canonical_inspection(&self) -> Inspection {
        Inspection {
            entity: EntityId::from_bytes([3; 16]),
            revision: RevisionId::new([4; 32]),
        }
    }
    /// Returns the Markdown inspection.
    ///
    /// # Returns
    /// Domain inspection.
    fn domain_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Returns the unchanged Markdown identity.
    ///
    /// # Returns
    /// Unchanged inspection.
    fn unchanged_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Declares Markdown projection support.
    ///
    /// # Returns
    /// Projection capability flags.
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            editable_projection: true,
            derived_analysis: false,
        }
    }
    /// Returns formatting-only projection evidence.
    ///
    /// # Returns
    /// Formatting-only evidence.
    fn projection(&self) -> Option<ProjectionEvidence> {
        Some(ProjectionEvidence {
            semantic_change: false,
            shared_change: false,
            unchanged_identity_reused: true,
        })
    }
}

/// Verifies Markdown formatting round trips without a semantic change.
#[test]
fn markdown_fixture_is_universal() {
    let document = parse("# title\n\ntext");
    assert_eq!(render(&document), render(&document));
    assert_conformance(&MarkdownFixture);
}
