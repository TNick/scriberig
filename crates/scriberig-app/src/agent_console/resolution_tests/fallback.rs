//! Verifies lower-scope model selection fallback behavior.

use super::*;

/// Resolves the nearest available scope through branch, session, project, user.
#[test]
fn lower_scope_order_is_branch_then_session_then_project_then_user_profile() {
    let branch = selection("local", "wire-beta", Some("minimal"));
    let session = selection("local", "wire-alpha", Some("high"));
    let project = selection("local", "wire-beta", Some("minimal"));
    let user = selection("local", "wire-alpha", Some("low"));
    let branch_result = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: Some(&branch),
            session: Some(&session),
            project: Some(&project),
            user_profile: Some(&user),
        },
        &upstream_models(),
        "runtime-default",
    )
    .expect("resolve branch scope");
    let session_result = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: None,
            session: Some(&session),
            project: Some(&project),
            user_profile: Some(&user),
        },
        &upstream_models(),
        "runtime-default",
    )
    .expect("resolve session scope");
    let project_result = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: None,
            session: None,
            project: Some(&project),
            user_profile: Some(&user),
        },
        &upstream_models(),
        "runtime-default",
    )
    .expect("resolve project scope");
    let user_result = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: None,
            session: None,
            project: None,
            user_profile: Some(&user),
        },
        &upstream_models(),
        "runtime-default",
    )
    .expect("resolve user-profile scope");

    assert_eq!(branch_result.1, "wire-beta");
    assert_eq!(session_result.1, "wire-alpha");
    assert_eq!(project_result.1, "wire-beta");
    assert_eq!(user_result.1, "wire-alpha");
}

/// Falls back through lower scopes and then the runtime model when needed.
#[test]
fn empty_narrow_scopes_fall_back_to_session_then_runtime() {
    let session = selection("local", "wire-beta", Some("minimal"));
    let resolved = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: None,
            session: Some(&session),
            project: None,
            user_profile: None,
        },
        &upstream_models(),
        "runtime-default",
    )
    .expect("resolve session scope");

    assert_eq!(
        resolved,
        (
            "local".to_owned(),
            "wire-beta".to_owned(),
            Some("minimal".to_owned()),
        )
    );
    assert_eq!(
        resolve_execution_selection(
            ExecutionSelectionSources {
                invocation: None,
                plan_node: None,
                branch: None,
                session: None,
                project: None,
                user_profile: None,
            },
            &upstream_models(),
            "runtime-default",
        )
        .expect("runtime fallback"),
        ("local".to_owned(), "runtime-default".to_owned(), None,)
    );
}

/// Keeps explicit branch and plan assignments above the profile preference.
#[test]
fn explicit_branch_and_plan_assignments_precede_agent_profile_preference() {
    use crate::agent_console::resolution::resolve_execution_selection_with_profile_preference;

    let profile = selection("local", "wire-alpha", Some("high"));
    let explicit = selection("local", "wire-beta", Some("minimal"));
    let user = selection("local", "runtime-default", None);
    let models = upstream_models();

    let branch = resolve_execution_selection_with_profile_preference(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: Some(&explicit),
            session: None,
            project: None,
            user_profile: Some(&user),
        },
        Some(&profile),
        &models,
        "runtime-default",
    )
    .expect("resolve explicit branch assignment");
    let plan_node = resolve_execution_selection_with_profile_preference(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: Some(&explicit),
            branch: None,
            session: None,
            project: None,
            user_profile: Some(&user),
        },
        Some(&profile),
        &models,
        "runtime-default",
    )
    .expect("resolve explicit plan assignment");

    assert_eq!(branch.1, "wire-beta");
    assert_eq!(plan_node.1, "wire-beta");
}

/// Places an active agent-profile preference before the generic user default.
#[test]
fn agent_profile_preference_precedes_general_user_profile_default() {
    use crate::agent_console::resolution::resolve_execution_selection_with_profile_preference;

    let profile = selection("local", "wire-alpha", Some("high"));
    let user = selection("local", "wire-beta", Some("minimal"));
    let resolved = resolve_execution_selection_with_profile_preference(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: None,
            session: None,
            project: None,
            user_profile: Some(&user),
        },
        Some(&profile),
        &upstream_models(),
        "runtime-default",
    )
    .expect("resolve agent-profile preference");

    assert_eq!(resolved.1, "wire-alpha");
    assert_eq!(resolved.2, Some(ThinkingLevel::High));
}
