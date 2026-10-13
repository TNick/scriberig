//! Checks durable self-hosting branch outcomes and stable result references.

use scriberig_session::{Branch, BranchStatus, Session};

/// Verifies durable child results expose bounded architecture and candidate evidence.
///
/// # Parameters
/// - `session`: Persisted session produced by the real self-hosting workflow.
/// - `super`: Parent PTY test module.
///
/// # Panics
/// Panics when either child is missing or the continuation omits candidate evidence.
pub(super) fn assert_nested_branch_results(session: &Session) {
    let continuation = session
        .branches
        .iter()
        .find(|branch| {
            branch.result.as_ref().is_some_and(|result| {
                result.summary.as_deref().is_some_and(|summary| {
                    summary.contains("validated PHP implementation candidate")
                })
            })
        })
        .expect("PHP continuation branch");
    let children = session
        .branches
        .iter()
        .filter(|branch| branch.parent == Some(continuation.id))
        .collect::<Vec<_>>();
    assert_eq!(
        children.len(),
        2,
        "expected architecture and implementation children"
    );
    assert_architecture_result(&children);
    assert_implementation_result(&children);

    let result = continuation
        .result
        .as_ref()
        .expect("continuation BranchResult");
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle.starts_with("candidate:"))
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle == "validation:passed")
    );
}

/// Verifies the architecture child retained its semantic discovery result.
///
/// # Parameters
/// - `children`: Child branches of the PHP continuation.
/// - `super`: Parent PTY test module.
///
/// # Panics
/// Panics when the architecture result is incomplete or lacks semantic references.
fn assert_architecture_result(children: &[&Branch]) {
    let branch = children
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose.contains("determine minimal extension points"))
        })
        .expect("architecture discovery child");
    let result = branch.result.as_ref().expect("architecture BranchResult");
    assert_eq!(branch.status, BranchStatus::Completed);
    assert_eq!(result.status, BranchStatus::Completed);
    assert_eq!(result.result_kind.as_deref(), Some("agent-child"));
    assert!(
        result
            .summary
            .as_deref()
            .is_some_and(|summary| summary.contains("ARCHITECTURE_MAP"))
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle.starts_with("source_digest:"))
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle.starts_with("entity:"))
    );
}

/// Verifies the implementation child retained exact candidate validation references.
///
/// # Parameters
/// - `children`: Child branches of the PHP continuation.
/// - `super`: Parent PTY test module.
///
/// # Panics
/// Panics when the implementation result omits its candidate or validation evidence.
fn assert_implementation_result(children: &[&Branch]) {
    let branch = children
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|purpose| purpose.contains("implement PHP ingestion"))
        })
        .expect("PHP implementation child");
    let result = branch.result.as_ref().expect("implementation BranchResult");
    assert_eq!(branch.status, BranchStatus::Completed);
    assert_eq!(result.status, BranchStatus::Completed);
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle.starts_with("candidate:"))
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle == "validation:passed")
    );
}
