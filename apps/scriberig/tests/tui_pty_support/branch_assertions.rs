//! Checks durable self-hosting branch outcomes and stable result references.

use scriberig_core::{ProjectId, SnapshotId};
use scriberig_session::{Branch, BranchResult, BranchStatus, Session};

#[path = "branch_assertions/validation.rs"]
mod validation;

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
    assert_no_shortcut_tools(continuation, &children);

    let result = continuation
        .result
        .as_ref()
        .expect("continuation BranchResult");
    assert_tool_trace(
        continuation,
        &[
            "runtime_capabilities",
            "semantic_find",
            "semantic_render",
            "delegate",
            "project_candidate",
        ],
    );
    assert!(
        continuation
            .tool_invocations
            .iter()
            .all(|tool| { tool.name != "project_apply_candidate" })
    );
    assert_scoped_reads(continuation, project, snapshot);
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

/// Verifies every PHP implementation tool call uses the bounded semantic and candidate API.
///
/// # Parameters
/// - `continuation`: Root continuation branch.
/// - `children`: Bounded architecture and implementation branches.
/// - `super`: Parent PTY support module.
///
/// # Panics
/// Panics when a branch attempts an unregistered or privileged shortcut tool.
fn assert_no_shortcut_tools(continuation: &Branch, children: &[&Branch]) {
    let allowed = [
        "runtime_capabilities",
        "semantic_find",
        "semantic_render",
        "delegate",
        "project_propose",
        "project_candidate",
        "project_resolve_dependencies",
        "project_validate_candidate",
    ];
    for branch in std::iter::once(continuation).chain(children.iter().copied()) {
        for invocation in &branch.tool_invocations {
            assert!(
                allowed.contains(&invocation.name.as_str()),
                "branch {} invoked shortcut or unregistered tool {}",
                branch.title,
                invocation.name
            );
        }
    }
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
    assert_tool_trace(branch, &["semantic_find", "semantic_render"]);
    assert_scoped_reads(branch, project, snapshot);
    assert!(branch.tool_invocations.iter().all(|tool| {
        !matches!(
            tool.name.as_str(),
            "project_propose" | "project_validate_candidate" | "project_apply_candidate"
        )
    }));
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
    assert_tool_trace(
        branch,
        &[
            "semantic_find",
            "semantic_render",
            "project_propose",
            "project_validate_candidate",
            "project_candidate",
            "project_resolve_dependencies",
        ],
    );
    validation::assert_dependency_resolution_precedes_validation(branch);
    validation::assert_validation_profiles(branch);
    assert_tool_input_scope(branch, "project_propose", project, snapshot);
    assert_scoped_reads(branch, project, snapshot);
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

/// Verifies required provider-facing calls have durable structured results.
///
/// # Parameters
/// - `branch`: Durable branch to inspect.
/// - `required`: Required tool names.
///
/// # Panics
/// Panics when an invocation, result, or JSON shape is missing.
fn assert_tool_trace(branch: &Branch, required: &[&str]) {
    for name in required {
        let invocation = branch
            .tool_invocations
            .iter()
            .find(|invocation| invocation.name == *name)
            .unwrap_or_else(|| panic!("branch {} omitted tool {name}", branch.title));
        let result = branch
            .tool_results
            .iter()
            .find(|result| result.invocation == invocation.id)
            .unwrap_or_else(|| panic!("tool {name} has no durable result"));
        assert!(!result.failed, "tool {name} failed in the success scenario");
        let input: serde_json::Value =
            serde_json::from_slice(&invocation.input).expect("structured tool input");
        let output: serde_json::Value =
            serde_json::from_slice(&result.output).expect("structured tool result");
        assert!(input.is_object());
        assert!(output.is_object());
    }
}

/// Verifies a candidate proposal is bound to its exact project snapshot.
///
/// # Parameters
/// - `branch`: Durable implementation branch.
/// - `name`: Proposal tool name.
/// - `project`: Expected semantic project identity.
/// - `snapshot`: Expected exact base snapshot.
///
/// # Panics
/// Panics when the durable proposal arguments omit or mismatch scope.
fn assert_tool_input_scope(branch: &Branch, name: &str, project: ProjectId, snapshot: SnapshotId) {
    let invocation = branch
        .tool_invocations
        .iter()
        .find(|invocation| invocation.name == name)
        .expect("project proposal invocation");
    let input: serde_json::Value =
        serde_json::from_slice(&invocation.input).expect("proposal JSON arguments");
    assert_eq!(input["project"], project.to_string());
    assert_eq!(input["base_snapshot"], snapshot.to_string());
}

/// Verifies all semantic read calls are bound to one exact project snapshot.
///
/// # Parameters
/// - `branch`: Durable branch to inspect.
/// - `project`: Expected semantic project identity.
/// - `snapshot`: Expected immutable snapshot.
///
/// # Panics
/// Panics when a semantic read omits or changes its exact scope.
fn assert_scoped_reads(branch: &Branch, project: ProjectId, snapshot: SnapshotId) {
    for invocation in branch.tool_invocations.iter().filter(|invocation| {
        matches!(
            invocation.name.as_str(),
            "semantic_find" | "semantic_render"
        )
    }) {
        let input: serde_json::Value =
            serde_json::from_slice(&invocation.input).expect("semantic tool JSON arguments");
        assert_eq!(input["project"], project.to_string());
        assert_eq!(input["snapshot"], snapshot.to_string());
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
