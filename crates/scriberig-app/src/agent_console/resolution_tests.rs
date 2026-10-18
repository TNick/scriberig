//! Verifies execution selection scope ordering and supported fallbacks.

use super::{ExecutionSelectionSources, resolve_execution_selection};
use crate::{ModelLibrary, SessionExecutionSelection, model_library_from_json};

/// Creates one session selection with the supplied profile and thinking id.
///
/// # Parameters
/// - `profile`: Curated profile identifier.
/// - `model`: Stable model identifier.
/// - `thinking`: Optional model-advertised level.
///
/// # Returns
/// A session selection for one scope.
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
/// # Returns
/// The validated test model library.
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
