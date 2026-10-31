//! Verifies reusable synthetic artifact fixtures and failure assertions.

use scriberig_core::{Provenance, ProvenanceSource};
use scriberig_domain_test_support::{
    ArtifactFixture, MaterializationFailureInjector, MaterializationMemberResult,
    PagedCollectionFixture, assert_exact_source_evidence, assert_no_partial_success,
};

/// Exercises one-file, compound, nested, and opaque fixture construction.
#[test]
fn artifact_fixtures_preserve_member_metadata_and_exact_digest() {
    let single = ArtifactFixture::single_file("notes.md", b"exact source".to_vec());
    assert_eq!(single.members.len(), 1);
    assert_eq!(
        single.digest("notes.md"),
        Some(*blake3::hash(b"exact source").as_bytes())
    );

    let compound = ArtifactFixture::multi_file_member("data.bin", b"member".to_vec()).with(
        ArtifactFixture::member("optional.idx", b"index".to_vec(), None, false, false),
    );
    assert_eq!(compound.members.len(), 2);
    assert!(!compound.members[1].required);

    let nested =
        ArtifactFixture::container_part("book.zip", "parts/sheet.xml", b"<sheet/>".to_vec());
    assert_eq!(nested.members[0].container.as_deref(), Some("book.zip"));

    let opaque = ArtifactFixture::opaque_member("opaque.bin", b"untouched".to_vec());
    assert!(opaque.members[0].opaque);
    assert!(opaque.members[0].required);
}

/// Exercises bounded paging and deterministic failure after committed members.
#[test]
fn large_collection_pages_and_materialization_failure_are_bounded() {
    let collection = PagedCollectionFixture::new((0..2_000).collect::<Vec<_>>());
    let (first, has_more) = collection.page(0, 500).expect("maximum page");
    assert_eq!(first.len(), 500);
    assert!(has_more);
    assert!(collection.page(0, 501).is_err());

    let failure = MaterializationFailureInjector::new(2);
    assert_eq!(failure.record_commit(), Ok(0));
    assert_eq!(failure.record_commit(), Ok(1));
    assert_eq!(failure.committed(), 2);
    assert_eq!(failure.record_commit().unwrap_err().committed_members, 2);
}

/// Verifies exact source evidence and rejects partial materialization success.
#[test]
fn conformance_assertions_check_source_evidence_and_required_members() {
    let provenance = Provenance {
        source: ProvenanceSource::Parser,
        producer: Some("fixture-parser".to_owned()),
        producer_version: Some("1".to_owned()),
        inputs: Vec::new(),
        profile: None,
        note: Some("synthetic evidence".to_owned()),
    };
    assert_exact_source_evidence(b"source", b"source", &provenance, &provenance);

    let complete = [
        MaterializationMemberResult {
            required: true,
            committed: true,
        },
        MaterializationMemberResult {
            required: false,
            committed: false,
        },
    ];
    assert_no_partial_success(true, &complete);

    let partial = [MaterializationMemberResult {
        required: true,
        committed: false,
    }];
    assert!(std::panic::catch_unwind(|| assert_no_partial_success(true, &partial)).is_err());
}
