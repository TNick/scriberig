//! Verifies execution selection scope ordering and supported fallbacks.

use super::{
    ExecutionSelectionSources, resolve_execution_selection,
    resolve_plan_node_execution_with_sources,
};
use crate::{ModelLibrary, SessionExecutionSelection, model_library_from_json};

/// Creates one session selection with the supplied profile and thinking id.
///
/// - `profile`: Curated profile identifier.
/// - `model`: Stable model identifier.
/// - `thinking`: Optional model-advertised level.
///
/// A session selection for one scope.
/// # Returns
/// The operation result.
fn selection(profile: &str, model: &str, thinking: Option<&str>) -> SessionExecutionSelection {
    SessionExecutionSelection {
        revision: 0,
        model_profile: Some(profile.to_owned()),
        model: Some(model.to_owned()),
        thinking_level: thinking.map(str::to_owned),
        history: Vec::new(),
    }
}

/// Creates two profiles with independent model-advertised thinking ids.
///
/// The validated test model library.
/// # Returns
/// The operation result.
fn library() -> ModelLibrary {
    model_library_from_json(
        Some(
            r#"{"models":[{"id":"alpha","display_name":"Alpha","provider_model_id":"wire-alpha","thinking_levels":[{"id":"measured","label":"Measured"},{"id":"deep","label":"Deep"}]},{"id":"beta","display_name":"Beta","provider_model_id":"wire-beta","thinking_levels":[{"id":"quick","label":"Quick"}]}]}"#,
        ),
        "fallback",
    )
    .expect("valid model library")
}

/// Resolves a more specific model and thinking choice before session defaults.
#[test]
fn invocation_scope_precedes_branch_session_project_and_user_profile() {
    let invocation = selection("alpha-profile", "alpha", Some("deep"));
    let branch = selection("beta-profile", "beta", Some("quick"));
    let session = selection("beta-profile", "beta", Some("quick"));
    let project = selection("alpha-profile", "alpha", Some("measured"));
    let user = selection("alpha-profile", "alpha", Some("measured"));

    let resolved = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: Some(&invocation),
            plan_node: None,
            branch: Some(&branch),
            session: Some(&session),
            project: Some(&project),
            user_profile: Some(&user),
        },
        &library(),
        "runtime-default",
    )
    .expect("resolve invocation scope");

    assert_eq!(resolved, ("wire-alpha".to_owned(), Some("deep".to_owned())));
}

/// Checks every configured scope in a table against the shared priority.
#[test]
fn each_selection_scope_resolves_its_configured_model() {
    let alpha = selection("alpha-profile", "alpha", None);
    let beta = selection("beta-profile", "beta", None);
    let cases = [
        (
            "invocation",
            Some(&alpha),
            None,
            None,
            None,
            None,
            None,
            "wire-alpha",
        ),
        (
            "plan node",
            None,
            Some(&beta),
            None,
            None,
            None,
            None,
            "wire-beta",
        ),
        (
            "branch",
            None,
            None,
            Some(&alpha),
            None,
            None,
            None,
            "wire-alpha",
        ),
        (
            "session",
            None,
            None,
            None,
            Some(&beta),
            None,
            None,
            "wire-beta",
        ),
        (
            "project",
            None,
            None,
            None,
            None,
            Some(&alpha),
            None,
            "wire-alpha",
        ),
        (
            "user",
            None,
            None,
            None,
            None,
            None,
            Some(&beta),
            "wire-beta",
        ),
    ];
    let library = library();
    for (name, invocation, plan_node, branch, session, project, user, expected) in cases {
        let actual = resolve_execution_selection(
            ExecutionSelectionSources {
                invocation,
                plan_node,
                branch,
                session,
                project,
                user_profile: user,
            },
            &library,
            "runtime-default",
        )
        .expect(name);
        assert_eq!(actual.0, expected, "{name}");
    }
}

/// Rejects unsupported invocation thinking before returning a provider model.
#[test]
fn unsupported_invocation_thinking_is_rejected() {
    let invocation = selection("alpha-profile", "alpha", Some("not-advertised"));
    assert!(
        resolve_execution_selection(
            ExecutionSelectionSources {
                invocation: Some(&invocation),
                plan_node: None,
                branch: None,
                session: None,
                project: None,
                user_profile: None,
            },
            &library(),
            "runtime-default",
        )
        .is_err()
    );
}

/// Keeps an invocation override above its node's stored assignment.
#[test]
fn invocation_override_wins_over_node_assignment() {
    let invocation = selection("alpha-profile", "alpha", Some("deep"));
    let node = scriberig_session::PlanNode {
        id: scriberig_core::EntityId::from_bytes([4; 16]),
        parent: None,
        title: "work".to_owned(),
        purpose: "work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: scriberig_session::PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        assignment: Some(scriberig_session::ModelAssignment {
            profile: "beta-profile".to_owned(),
            model: "beta".to_owned(),
            thinking_level: Some("quick".to_owned()),
        }),
        claim: None,
        claim_generation: 0,
    };
    let resolved = resolve_plan_node_execution_with_sources(
        &node,
        ExecutionSelectionSources {
            invocation: Some(&invocation),
            plan_node: None,
            branch: None,
            session: None,
            project: None,
            user_profile: None,
        },
        &library(),
        "runtime-default",
    )
    .expect("invocation override");
    assert_eq!(resolved.provider_model_id, "wire-alpha");
    assert_eq!(resolved.thinking_level.as_deref(), Some("deep"));
}

/// Resolves the nearest available scope through branch, session, project, user.
#[test]
fn lower_scope_order_is_branch_then_session_then_project_then_user_profile() {
    let branch = selection("beta-profile", "beta", Some("quick"));
    let session = selection("alpha-profile", "alpha", Some("deep"));
    let project = selection("beta-profile", "beta", Some("quick"));
    let user = selection("alpha-profile", "alpha", Some("measured"));
    let library = library();
    let branch_result = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: Some(&branch),
            session: Some(&session),
            project: Some(&project),
            user_profile: Some(&user),
        },
        &library,
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
        &library,
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
        &library,
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
        &library,
        "runtime-default",
    )
    .expect("resolve user-profile scope");

    assert_eq!(branch_result.0, "wire-beta");
    assert_eq!(session_result.0, "wire-alpha");
    assert_eq!(project_result.0, "wire-beta");
    assert_eq!(user_result.0, "wire-alpha");
}

/// Falls back through lower scopes and then the runtime model when needed.
#[test]
fn empty_narrow_scopes_fall_back_to_session_then_runtime() {
    let session = selection("beta-profile", "beta", None);
    let resolved = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: None,
            plan_node: None,
            branch: None,
            session: Some(&session),
            project: None,
            user_profile: None,
        },
        &library(),
        "runtime-default",
    )
    .expect("resolve session scope");

    assert_eq!(resolved, ("wire-beta".to_owned(), Some("quick".to_owned())));
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
            &library(),
            "runtime-default",
        )
        .expect("runtime fallback"),
        ("runtime-default".to_owned(), None)
    );
}
