//! Checks the language-independent code adapter through the shared harness.

use scriberig_core::{EntityId, RevisionId};
use scriberig_domain_code::code_schema;
use scriberig_domain_test_support::{ConformanceFixture, Inspection, assert_conformance};

/// Supplies the representative code-domain conformance case.
struct CodeFixture;

impl ConformanceFixture for CodeFixture {
    /// Returns the fixture domain name.
    ///
    /// Domain name.
    /// # Returns
    /// The operation result.
    fn domain(&self) -> &'static str {
        "code"
    }
    /// Returns the canonical code identity.
    ///
    /// Canonical inspection.
    /// # Returns
    /// The operation result.
    fn canonical_inspection(&self) -> Inspection {
        Inspection {
            entity: EntityId::from_bytes([1; 16]),
            revision: RevisionId::new([2; 32]),
        }
    }
    /// Returns the code-domain inspection.
    ///
    /// Domain inspection.
    /// # Returns
    /// The operation result.
    fn domain_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
    /// Returns the unchanged code identity.
    ///
    /// Unchanged inspection.
    /// # Returns
    /// The operation result.
    fn unchanged_inspection(&self) -> Inspection {
        self.canonical_inspection()
    }
}

/// Verifies code schema construction and universal identity conformance.
#[test]
fn code_fixture_is_universal() {
    assert_ne!(code_schema().entity_types.len(), 0);
    assert_conformance(&CodeFixture);
}
