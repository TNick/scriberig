//! Tests comment recovery and deterministic source-evidence grouping.
use scriberig_domain_rust::{code_groups::initial_groups, comments::recover_comments};

/// Recovers authored comments without treating line numbers as identities.
#[test]
fn comments_are_first_class_source_evidence() {
    let comments = recover_comments("/// docs\nfn value() {}\n// note");
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].value.text, "docs");
}

/// Code groups are deterministic and have no generated descriptions.
#[test]
fn groups_are_deterministic() {
    let first = initial_groups(2);
    let second = initial_groups(2);
    assert_eq!(first, second);
    assert!(first.iter().all(|group| group.description.is_none()));
}
