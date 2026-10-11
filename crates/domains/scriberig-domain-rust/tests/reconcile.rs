//! Verifies Rust syntax identity reuse and local reconciliation changes.

use scriberig_core::{EntityId, SnapshotId};
use scriberig_domain_rust::{
    CanonicalRustItem, RustChange, RustItem, RustMigrationProposal, RustReconciliation, parse,
    reconcile, reconcile_with_comments,
};
use sr_change::Operation;

/// Creates canonical items with deterministic test identities.
///
/// # Parameters
/// - `source`: Rust source to parse.
/// - `ids`: Entity identity bytes in item order.
///
/// # Returns
/// Parsed canonical items.
fn canonical(source: &str, ids: &[u8]) -> Vec<CanonicalRustItem> {
    let file = parse(source).expect("canonical Rust source");
    file.items
        .into_iter()
        .enumerate()
        .map(|(position, item)| CanonicalRustItem {
            id: EntityId::new([ids[position]; 16]),
            item,
            position,
        })
        .collect()
}

/// Returns a migration proposal from a reconciliation result.
///
/// # Parameters
/// - `result`: Rust reconciliation result.
///
/// # Returns
/// The proposal, or panics when a test unexpectedly receives another state.
fn proposal(result: RustReconciliation) -> Box<RustMigrationProposal> {
    let RustReconciliation::MigrationProposal(value) = result else {
        panic!("expected migration proposal");
    };
    value
}

/// Verifies unchanged parsed syntax produces no migration.
#[test]
fn unchanged_source_is_a_no_op() {
    let source = "fn add(a: i32) -> i32 { a + 1 }";
    let base = canonical(source, &[1]);
    let candidate = parse(source).expect("candidate");
    assert!(matches!(
        reconcile(SnapshotId::new([9; 32]), &base, &candidate, "lib.rs"),
        RustReconciliation::NoSemanticChange
    ));
}

/// Verifies an expression change stays local to the matched function.
#[test]
fn expression_change_reuses_function_identity() {
    let base = canonical("fn add(a: i32) -> i32 { a + 1 }", &[1]);
    let candidate = parse("fn add(a: i32) -> i32 { a * 1 }").expect("candidate");
    let proposal = proposal(reconcile(
        SnapshotId::new([9; 32]),
        &base,
        &candidate,
        "lib.rs",
    ));
    assert!(matches!(
        proposal.operations.as_slice(),
        [Operation::ReplaceRevision { entity, .. }] if *entity == base[0].id
    ));
    assert!(proposal.changes.iter().any(|change| matches!(
        change,
        RustChange::ReplaceStatement { function, .. } if *function == base[0].id
    )));
}

/// Verifies formatting-only changes remain semantically stable.
#[test]
fn formatting_change_is_a_no_op() {
    let base = canonical("fn add(a: i32) -> i32 { a + 1 }", &[1]);
    let candidate = parse("fn add(a: i32) -> i32 {\n    a + 1\n}\n").expect("candidate");
    assert!(matches!(
        reconcile(SnapshotId::new([9; 32]), &base, &candidate, "lib.rs"),
        RustReconciliation::NoSemanticChange
    ));
}

/// Verifies duplicate declaration names produce explicit ambiguity.
#[test]
fn duplicate_functions_are_ambiguous() {
    let source = "fn same() {}\nfn same() {}";
    let base = canonical(source, &[1, 2]);
    let candidate = parse("fn same() { 1 }").expect("candidate");
    assert!(matches!(
        reconcile(SnapshotId::new([9; 32]), &base, &candidate, "lib.rs"),
        RustReconciliation::Ambiguous(_)
    ));
}

/// Verifies macro changes replace the invocation as one safe unit.
#[test]
fn macro_changes_replace_the_macro_unit() {
    let base = canonical("make!(one);", &[1]);
    let candidate = parse("make!(two);").expect("candidate");
    let proposal = proposal(reconcile(
        SnapshotId::new([9; 32]),
        &base,
        &candidate,
        "lib.rs",
    ));
    assert!(matches!(
        proposal.operations[0],
        Operation::ReplaceRevision { .. }
    ));
    assert!(matches!(base[0].item, RustItem::Macro(_)));
}

/// Verifies comment edits are represented without replacing unrelated syntax.
#[test]
fn comment_edit_is_explicit() {
    let base_source = "// old\nfn add() {}";
    let candidate_source = "// new\nfn add() {}";
    let base = canonical(base_source, &[1]);
    let base_file = parse(base_source).expect("base");
    let candidate = parse(candidate_source).expect("candidate");
    let proposal = proposal(reconcile_with_comments(
        SnapshotId::new([9; 32]),
        &base,
        &base_file.comments,
        &candidate,
        "lib.rs",
    ));
    assert!(
        proposal
            .changes
            .iter()
            .any(|change| matches!(change, RustChange::CommentChange))
    );
    assert_eq!(proposal.operations.len(), 0);
}
