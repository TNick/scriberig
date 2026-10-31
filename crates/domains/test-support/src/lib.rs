//! Reusable, domain-neutral assertions for adapter conformance tests.

mod artifact;
mod conformance;
mod fixture;

pub use artifact::{
    ArtifactFixture, ArtifactFixtureMember, FailureInjection, MaterializationFailureInjector,
    MaterializationMemberResult, PagedCollectionFixture, assert_exact_source_evidence,
    assert_no_partial_success,
};
pub use conformance::assert_conformance;
pub use fixture::{
    CapabilitySet, ConformanceFixture, DerivedEvidence, Inspection, ProjectionEvidence,
};
