//! Implements readable assertions for the shared domain invariants.

use crate::{CapabilitySet, ConformanceFixture};

/// Runs all applicable domain conformance assertions for one fixture.
///
/// # Parameters
/// - `fixture`: Domain-owned fixture implementing the generic contract.
///
/// # Panics
/// Panics with the domain name, capability, and expected invariant when a
/// fixture violates an applicable invariant.
pub fn assert_conformance<F: ConformanceFixture>(fixture: &F) {
    let domain = fixture.domain();
    let canonical = fixture.canonical_inspection();
    let specialized = fixture.domain_inspection();
    assert_eq!(
        canonical.entity, specialized.entity,
        "{domain}/inspection: domain inspection must retain universal entity identity"
    );
    assert_eq!(
        canonical.revision, specialized.revision,
        "{domain}/inspection: domain inspection must retain exact revision"
    );

    let unchanged = fixture.unchanged_inspection();
    assert_eq!(
        canonical.entity, unchanged.entity,
        "{domain}/identity: unchanged semantic entity must be reusable"
    );
    assert_eq!(
        canonical.revision, unchanged.revision,
        "{domain}/revision: unchanged semantic revision must be reusable"
    );

    let capabilities = fixture.capabilities();
    assert_optional_capabilities(domain, capabilities, fixture);
}

/// Checks optional projection and derived-data boundaries only when declared.
fn assert_optional_capabilities<F: ConformanceFixture>(
    domain: &str,
    capabilities: CapabilitySet,
    fixture: &F,
) {
    if capabilities.editable_projection {
        let projection = fixture.projection().unwrap_or_else(|| {
            panic!(
                "{domain}/projection: fixture declares editable projection but supplies no evidence"
            )
        });
        if projection.semantic_change {
            assert!(
                projection.shared_change,
                "{domain}/projection: semantic reconciliation must produce a shared change object"
            );
        } else {
            assert!(
                projection.unchanged_identity_reused,
                "{domain}/projection: formatting-only change must reuse identity and revision"
            );
        }
    }

    if capabilities.derived_analysis {
        let derived = fixture.derived().unwrap_or_else(|| {
            panic!("{domain}/derived: fixture declares derived analysis but supplies no evidence")
        });
        assert_eq!(
            derived.input.entity_id,
            fixture.canonical_inspection().entity,
            "{domain}/derived: provenance must name the canonical source entity"
        );
        assert_eq!(
            derived.input.revision_id,
            fixture.canonical_inspection().revision,
            "{domain}/derived: provenance must name the exact source revision"
        );
        assert!(
            derived.provenance.inputs.contains(&derived.input),
            "{domain}/derived: provenance must retain the exact source reference"
        );
        assert!(
            derived.canonical_preserved,
            "{domain}/derived: discarding derived data must preserve canonical state"
        );
        assert!(
            derived.generated_summary_is_derived,
            "{domain}/derived: generated summaries must remain derived"
        );
    }
}
