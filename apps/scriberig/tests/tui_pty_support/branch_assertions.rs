//! Checks durable self-hosting branch outcomes and stable result references.

use scriberig_core::{ProjectId, SnapshotId};
use scriberig_session::{Branch, BranchResult, BranchStatus, Session};

/// Verifies durable child results expose bounded architecture and candidate evidence.
///
/// # Parameters
/// - `session`: Persisted session produced by the real self-hosting workflow.
/// - `project`: Expected semantic project identity.
/// - `snapshot`: Exact base snapshot expected in both child results.
/// - `super`: Parent PTY test module.
///
/// # Panics
/// Panics when either child is missing or the continuation omits candidate evidence.
pub(super) fn assert_nested_branch_results(
    session: &Session,
    project: ProjectId,
    snapshot: SnapshotId,
) {
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
    assert_architecture_result(&children, project, snapshot);
    assert_implementation_result(&children, project, snapshot);

    let result = continuation
        .result
        .as_ref()
        .expect("continuation BranchResult");
    assert_scope_handles(result, project, snapshot);
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
/// - `project`: Exact project expected in child references.
/// - `snapshot`: Exact immutable base expected in child references.
/// - `super`: Parent PTY test module.
///
/// # Panics
/// Panics when the architecture result is incomplete or lacks semantic references.
fn assert_architecture_result(children: &[&Branch], project: ProjectId, snapshot: SnapshotId) {
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
    assert_scope_handles(result, project, snapshot);
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
/// - `project`: Exact project expected in child references.
/// - `snapshot`: Exact immutable base expected in child references.
/// - `super`: Parent PTY test module.
///
/// # Panics
/// Panics when the implementation result omits its candidate or validation evidence.
fn assert_implementation_result(children: &[&Branch], project: ProjectId, snapshot: SnapshotId) {
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
    assert_scope_handles(result, project, snapshot);
    assert!(branch.turns.first().is_some_and(|turn| {
        turn.human.as_ref().is_some_and(|prompt| {
            prompt.text.contains("Compact architecture BranchResult")
                && prompt.text.contains("stable refs:")
        })
    }));
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
    for path in [
        "Cargo.toml",
        "crates/scriberig-project/Cargo.toml",
        "crates/scriberig-project/src/scan.rs",
        "crates/scriberig-project/src/adapters.rs",
        "crates/scriberig-project/src/support.rs",
    ] {
        assert!(
            result
                .result_handles
                .iter()
                .any(|handle| handle == &format!("path:{path}")),
            "implementation branch omitted re-inspected source path {path}"
        );
    }
}

/// Verifies a child result retains the exact semantic project and snapshot scope.
///
/// # Parameters
/// - `result`: Durable branch result to inspect.
/// - `project`: Expected project identity.
/// - `snapshot`: Expected immutable scope.
/// - `super`: Parent PTY test module.
///
/// # Panics
/// Panics when the result omits either exact scope handle.
fn assert_scope_handles(result: &BranchResult, project: ProjectId, snapshot: SnapshotId) {
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle == &format!("project:{project}"))
    );
    assert!(
        result
            .result_handles
            .iter()
            .any(|handle| handle == &format!("snapshot:{snapshot}"))
    );
}
