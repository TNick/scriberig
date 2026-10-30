//! Routes durable plan slash commands to `ScribeRig` session operations.

use crate::effect::Effect;
use crate::state::TuiState;

use super::super::plan;

/// Routes one plan command to shared durable plan operations.
///
/// - `state`: Current `ScribeRig` session and presentation state.
/// - `argument`: Text following `/plan`.
///
/// Effects for the requested plan operation or local validation.
/// # Returns
/// Effects for a recognized plan command.
pub(super) fn plan_command(state: &mut TuiState, argument: &str) -> Vec<Effect> {
    if argument.is_empty() {
        return invalid_plan_command(
            state,
            "Use /plan <goal>, /plan show, or /plan start|pause|resume|stop|complete.",
        );
    }
    if argument == "show" {
        return show_plan(state);
    }
    if let Some(reason) = argument.strip_prefix("replan ").map(str::trim)
        && !reason.is_empty()
    {
        return replan(state, reason);
    }
    if let Some(action) = lifecycle_action(argument) {
        return change_lifecycle(state, action);
    }
    create_plan(state, argument)
}

/// Shows a selected durable plan summary.
///
/// - `state`: Current session and presentation state.
///
/// A no-op effect after updating the shell message or validation message.
/// # Returns
/// Effects for the plan summary request.
fn show_plan(state: &mut TuiState) -> Vec<Effect> {
    let Some(session) = state.session_snapshot.as_ref() else {
        return invalid_plan_command(state, "Load a session before showing a plan.");
    };
    let Some(plan) = selected_plan(state, session, true) else {
        return invalid_plan_command(state, "This session has no plan yet.");
    };
    let completed = plan
        .nodes
        .iter()
        .filter(|node| node.result.is_some())
        .count();
    state.shell_message = format!(
        "Plan: {} · {:?} · revision {} · {}/{} results · {} dependencies",
        plan.goal,
        plan.status,
        plan.revision,
        completed,
        plan.nodes.len(),
        plan.dependencies.len()
    );
    vec![Effect::None]
}

/// Creates one additive plan revision from a replan reason.
///
/// - `state`: Current session and selected plan state.
/// - `reason`: User-provided purpose for the addition.
///
/// A plan replan effect or local validation response.
/// # Returns
/// Effects for the requested plan revision.
fn replan(state: &mut TuiState, reason: &str) -> Vec<Effect> {
    let Some(session) = state.session_snapshot.as_ref() else {
        return invalid_plan_command(state, "Load a session before revising a plan.");
    };
    let Some(plan) = selected_plan(state, session, false) else {
        return invalid_plan_command(state, "Create a plan before revising it.");
    };
    let title = reason.lines().next().unwrap_or("Replanned work");
    let title = title.chars().take(80).collect::<String>();
    vec![Effect::Replan(scriberig_app::PlanReplanRequest {
        session: session.id,
        plan: plan.id,
        expected_revision: plan.revision,
        additions: vec![scriberig_app::PlanNodeDraft {
            key: "replan-addition".to_owned(),
            parent_index: None,
            title,
            purpose: reason.to_owned(),
            expected_result: Some(scriberig_session::ExpectedResult {
                kind: "summary".to_owned(),
                requires_handle: false,
            }),
            semantic_scope: Vec::new(),
            assignment: None,
        }],
        dependencies: Vec::new(),
        removed_dependencies: Vec::new(),
        superseded: Vec::new(),
        assignment_changes: Vec::new(),
        preserved_terminal_nodes: plan
            .nodes
            .iter()
            .filter(|node| node.result.is_some())
            .map(|node| node.id)
            .collect(),
        reason: reason.to_owned(),
    })]
}

/// Applies a plan lifecycle action to the selected durable plan.
///
/// - `state`: Current session and selected plan state.
/// - `action`: Requested lifecycle transition.
///
/// A lifecycle effect or local validation response.
/// # Returns
/// Effects for the requested lifecycle transition.
fn change_lifecycle(
    state: &mut TuiState,
    action: scriberig_app::PlanLifecycleAction,
) -> Vec<Effect> {
    let Some(session) = state.session_snapshot.as_ref() else {
        return invalid_plan_command(state, "Load a session before controlling a plan.");
    };
    let Some(plan) = selected_plan(state, session, false) else {
        return invalid_plan_command(state, "Create a plan before changing its lifecycle.");
    };
    vec![Effect::PlanLifecycle {
        request: scriberig_app::PlanLifecycleRequest {
            session: session.id,
            plan: plan.id,
            expected_revision: plan.revision,
        },
        action,
    }]
}

/// Creates a durable plan from the active conversation target.
///
/// - `state`: Current conversation target.
/// - `argument`: User-provided goal and node draft.
///
/// A plan creation effect or local validation response.
/// # Returns
/// Effects for the requested plan creation.
fn create_plan(state: &mut TuiState, argument: &str) -> Vec<Effect> {
    let crate::state::ConversationTarget::Existing {
        session,
        reply_anchor,
    } = state.conversation_composer.target
    else {
        return invalid_plan_command(state, "Start a conversation before creating a plan.");
    };
    let draft = match plan::parse_plan_argument(argument) {
        Ok(draft) => draft,
        Err(message) => {
            state.conversation_composer.error = Some(message);
            return vec![Effect::None];
        }
    };
    vec![Effect::CreatePlan(scriberig_app::PlanCreateRequest {
        session,
        anchor: reply_anchor,
        goal: draft.goal,
        nodes: draft.nodes,
        dependencies: draft.dependencies,
    })]
}

/// Finds the active lifecycle action encoded by a plan command.
///
/// - `argument`: Text following `/plan`.
///
/// The requested lifecycle action, if the argument names one.
/// # Returns
/// A supported plan lifecycle action.
fn lifecycle_action(argument: &str) -> Option<scriberig_app::PlanLifecycleAction> {
    match argument {
        "start" => Some(scriberig_app::PlanLifecycleAction::Start),
        "pause" => Some(scriberig_app::PlanLifecycleAction::Pause),
        "resume" => Some(scriberig_app::PlanLifecycleAction::Resume),
        "stop" => Some(scriberig_app::PlanLifecycleAction::Stop),
        "complete" => Some(scriberig_app::PlanLifecycleAction::Complete),
        _ => None,
    }
}

/// Finds the selected plan or the most recent plan in a session.
///
/// - `state`: Current plan and conversation cursor.
/// - `session`: Durable session snapshot containing plans.
/// - `use_plan_cursor`: Whether the plan view cursor takes precedence.
///
/// The selected plan, falling back to the latest plan.
/// # Returns
/// A plan in the session when one exists.
fn selected_plan<'a>(
    state: &TuiState,
    session: &'a scriberig_session::Session,
    use_plan_cursor: bool,
) -> Option<&'a scriberig_session::Plan> {
    let selected = if use_plan_cursor && state.primary_view == crate::selection::PrimaryView::Plan {
        state.plan_cursor.map(|selection| selection.plan)
    } else {
        match state.conversation_cursor {
            Some(crate::view::conversation::ConversationSource::Plan(plan)) => Some(plan),
            Some(crate::view::conversation::ConversationSource::PlanNode(node)) => session
                .plans
                .iter()
                .find(|plan| plan.nodes.iter().any(|item| item.id == node))
                .map(|plan| plan.id),
            _ => None,
        }
    };
    selected
        .and_then(|id| session.plans.iter().find(|plan| plan.id == id))
        .or_else(|| session.plans.last())
}

/// Stores a plan validation message and produces a no-op effect.
///
/// - `state`: Presentation state receiving the message.
/// - `message`: User-facing validation detail.
///
/// A no-op effect.
/// # Returns
/// A single no-op effect for local command validation.
fn invalid_plan_command(state: &mut TuiState, message: &str) -> Vec<Effect> {
    state.conversation_composer.error = Some(message.to_owned());
    vec![Effect::None]
}
