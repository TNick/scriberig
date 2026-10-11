//! Reusable, domain-neutral assertions for adapter conformance tests.

mod conformance;
mod fixture;

pub use conformance::assert_conformance;
pub use fixture::{
    CapabilitySet, ConformanceFixture, DerivedEvidence, Inspection, ProjectionEvidence,
};
