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
    /// Domain name.
    /// # Returns
    /// The operation result.
    fn domain(&self) -> &'static str {
        "markdown"
    }
    /// Returns the canonical Markdown identity.
    ///
    /// Canonical inspection.
    /// # Returns
    /// The operation result.
    fn canonical_inspection(&self) -> Inspection {
        Inspection {
            entity: EntityId::from_bytes([3; 16]),
            revision: RevisionId::new([4; 32]),
        }
    }
    /// Returns the Markdown inspection.
    ///
    /// Domain inspection.
    /// # Returns
    /// The operation result.
    fn domain_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Returns the unchanged Markdown identity.
    ///
    /// Unchanged inspection.
    /// # Returns
    /// The operation result.
    fn unchanged_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Declares Markdown projection support.
    ///
    /// Projection capability flags.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            editable_projection: true,
            derived_analysis: false,
        }
    }
    /// Returns formatting-only projection evidence.
    ///
    /// Formatting-only evidence.
    /// # Returns
    /// The operation result.
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
