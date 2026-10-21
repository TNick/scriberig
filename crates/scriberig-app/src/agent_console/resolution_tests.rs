//! Verifies execution selection scope ordering and supported fallbacks.

use super::{
    ExecutionSelectionSources, resolve_execution_selection,
    resolve_plan_node_execution_with_sources,
};
use crate::SessionExecutionSelection;
use rpi_ai::{Api, Model, ThinkingLevel};
use std::collections::BTreeMap;

/// Builds the upstream catalog corresponding to the safe test projection.
///
/// The provider models accepted by assignment resolution.
/// # Returns
/// The operation result.
fn upstream_models() -> Vec<Model> {
    vec![
        Model::new(
            "wire-alpha",
            "Alpha",
            Api::AnthropicMessages,
            "local",
            "https://example.test",
        ),
        Model::new(
            "wire-beta",
            "Beta",
            Api::AnthropicMessages,
            "local",
            "https://example.test",
        ),
        Model::new(
            "runtime-default",
            "Runtime fallback",
            Api::AnthropicMessages,
            "local",
            "https://example.test",
        ),
    ]
}

/// Creates one session selection with the supplied provider/model pair.
///
/// - `provider_id`: Upstream provider identifier.
/// - `model_id`: Provider-scoped upstream model identifier.
/// - `thinking`: Optional model-advertised level.
///
/// A session selection for one scope.
/// # Returns
/// The operation result.
fn selection(
    provider_id: &str,
    model_id: &str,
    thinking: Option<&str>,
) -> SessionExecutionSelection {
    SessionExecutionSelection {
        revision: 0,
        provider_id: Some(provider_id.to_owned()),
        model_id: Some(model_id.to_owned()),
        thinking_level: thinking.map(str::to_owned),
        history: Vec::new(),
    }
}

/// Resolves a more specific model and thinking choice before session defaults.
#[test]
fn invocation_scope_precedes_branch_session_project_and_user_profile() {
    let invocation = selection("local", "wire-alpha", Some("high"));
    let branch = selection("local", "wire-beta", Some("minimal"));
    let session = selection("local", "wire-beta", Some("minimal"));
    let project = selection("local", "wire-alpha", Some("low"));
    let user = selection("local", "wire-alpha", Some("low"));

    let resolved = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: Some(&invocation),
            plan_node: None,
            branch: Some(&branch),
            session: Some(&session),
            project: Some(&project),
            user_profile: Some(&user),
        },
        &upstream_models(),
        "runtime-default",
    )
    .expect("resolve invocation scope");

    assert_eq!(
        resolved,
        (
            "local".to_owned(),
            "wire-alpha".to_owned(),
            Some("high".to_owned()),
        )
    );
}

/// Checks every configured scope in a table against the shared priority.
#[test]
fn each_selection_scope_resolves_its_configured_model() {
    let alpha = selection("local", "wire-alpha", None);
    let beta = selection("local", "wire-beta", None);
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
            &upstream_models(),
            "runtime-default",
        )
        .expect(name);
        assert_eq!(actual.1, expected, "{name}");
    }
}

/// Rejects unsupported invocation thinking before returning a provider model.
#[test]
fn unsupported_invocation_thinking_is_rejected() {
    let invocation = selection("local", "wire-alpha", Some("not-advertised"));
    let error = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: Some(&invocation),
            plan_node: None,
            branch: None,
            session: None,
            project: None,
            user_profile: None,
        },
        &upstream_models(),
        "runtime-default",
    )
    .expect_err("reject noncanonical persisted thinking");
    assert!(matches!(
        error,
        crate::ApplicationError::InvalidRequest {
            operation,
            message,
        } if operation == "agent_start"
            && message == "thinking level id is not supported by upstream"
    ));
}

/// Lets upstream clamp a canonical level that this model does not advertise.
#[test]
fn valid_unadvertised_thinking_is_clamped_by_upstream() {
    let invocation = selection("local", "wire-alpha", Some("max"));
    let resolved = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: Some(&invocation),
            plan_node: None,
            branch: None,
            session: None,
            project: None,
            user_profile: None,
        },
        &upstream_models(),
        "runtime-default",
    )
    .expect("canonical unsupported level is clamped by upstream");

    assert_eq!(resolved.2.as_deref(), Some("high"));
}

/// Re-resolves the retained thinking choice against the newly selected model.
#[test]
fn model_switch_uses_new_upstream_thinking_support_and_clamps() {
    let mut models = upstream_models();
    let mut beta_thinking = BTreeMap::new();
    beta_thinking.insert(ThinkingLevel::High, None);
    beta_thinking.insert(ThinkingLevel::Medium, Some("balanced".to_owned()));
    models[1].thinking_level_map = Some(beta_thinking);

    let previous = selection("local", "wire-alpha", Some("high"));
    let previous_resolution = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: Some(&previous),
            plan_node: None,
            branch: None,
            session: None,
            project: None,
            user_profile: None,
        },
        &models,
        "runtime-default",
    )
    .expect("resolve prior model selection");
    assert_eq!(previous_resolution.2.as_deref(), Some("high"));

    let switched = selection("local", "wire-beta", previous_resolution.2.as_deref());
    let switched_resolution = resolve_execution_selection(
        ExecutionSelectionSources {
            invocation: Some(&switched),
            plan_node: None,
            branch: None,
            session: None,
            project: None,
            user_profile: None,
        },
        &models,
        "runtime-default",
    )
    .expect("resolve switched model from upstream catalog");

    assert_eq!(switched_resolution.1, "wire-beta");
    assert_eq!(switched_resolution.2.as_deref(), Some("medium"));
}

/// Keeps an invocation override above its node's stored assignment.
#[test]
fn invocation_override_wins_over_node_assignment() {
    let invocation = selection("local", "wire-alpha", Some("high"));
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
            provider_id: "local".to_owned(),
            model_id: "wire-beta".to_owned(),
            thinking_level: Some("minimal".to_owned()),
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
        &upstream_models(),
        "runtime-default",
    )
    .expect("invocation override");
    assert_eq!(resolved.provider_id, "local");
    assert_eq!(resolved.model_id, "wire-alpha");
    assert_eq!(resolved.thinking_level, Some(ThinkingLevel::High));
}

#[path = "resolution_tests/fallback.rs"]
mod fallback;
