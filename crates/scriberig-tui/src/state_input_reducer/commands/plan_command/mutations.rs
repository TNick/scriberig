//! Builds revision-checked plan action requests from focused slash commands.

use crate::{effect::Effect, state::TuiState};

use super::invalid_plan_command;

/// Adds ordinary next-step work beneath the selected plan.
///
/// - `state`: Current session and plan selection.
/// - `work`: Work title and purpose separated by a colon.
///
/// A shared next-step effect or local validation message.
/// # Returns
/// Effects for the requested plan action.
pub(super) fn add_next_step(state: &mut TuiState, work: &str) -> Vec<Effect> {
    let Some((title, purpose)) = parse_work(work) else {
        return invalid_plan_command(state, "Use /plan next <title>: <purpose>.");
    };
    let Some((session, plan)) = selected_plan(state) else {
        return invalid_plan_command(state, "Select a durable plan before adding a next step.");
    };
    vec![Effect::AddPlanNextStep(
        scriberig_app::PlanNextStepRequest {
            session: session.id,
            plan: plan.id,
            expected_revision: plan.revision,
            title,
            purpose,
            expected_result: Some(scriberig_session::ExpectedResult {
                kind: "summary".to_owned(),
                requires_handle: false,
            }),
            semantic_scope: Vec::new(),
            assignment: None,
        },
    )]
}

/// Adds required children beneath the selected plan node.
///
/// - `state`: Current session and stable node selection.
/// - `work`: Child title and purpose separated by a colon.
///
/// A shared decomposition effect or local validation message.
/// # Returns
/// Effects for the requested plan action.
pub(super) fn decompose_selected_node(state: &mut TuiState, work: &str) -> Vec<Effect> {
    let Some((title, purpose)) = parse_work(work) else {
        return invalid_plan_command(state, "Use /plan child <title>: <purpose>.");
    };
    let Some((session, plan)) = selected_plan(state) else {
        return invalid_plan_command(state, "Select a durable plan before adding a child.");
    };
    let Some(parent) = state.plan_cursor.and_then(|selection| selection.node) else {
        return invalid_plan_command(state, "Select a plan node before adding a child.");
    };
    vec![Effect::DecomposePlanNode(
        scriberig_app::PlanDecomposeRequest {
            session: session.id,
            plan: plan.id,
            parent,
            expected_revision: plan.revision,
            children: vec![scriberig_app::PlanChildDraft {
                title,
                purpose,
                expected_result: Some(scriberig_session::ExpectedResult {
                    kind: "summary".to_owned(),
                    requires_handle: false,
                }),
                semantic_scope: Vec::new(),
                assignment: None,
            }],
            required: true,
        },
    )]
}

/// Retries the selected failed node through shared application semantics.
///
/// - `state`: Current session and stable plan/node selection.
///
/// A retry effect or local validation message.
/// # Returns
/// Effects for the requested plan action.
pub(super) fn retry_selected_node(state: &mut TuiState) -> Vec<Effect> {
    let Some((session, plan)) = selected_plan(state) else {
        return invalid_plan_command(state, "Select a durable plan before retrying work.");
    };
    let Some(node) = state.plan_cursor.and_then(|selection| selection.node) else {
        return invalid_plan_command(state, "Select a failed plan node before retrying it.");
    };
    vec![Effect::RetryPlanNode(scriberig_app::PlanRetryRequest {
        session: session.id,
        plan: plan.id,
        node,
        expected_revision: plan.revision,
    })]
}

/// Waives a gate with an explicit reason and shared actor provenance.
///
/// - `state`: Current session and stable plan selection.
/// - `waiver`: Gate identity followed by a user-authored reason.
///
/// A typed semantic mutation effect or local validation message.
/// # Returns
/// Effects for the requested plan action.
pub(super) fn waive_gate(state: &mut TuiState, waiver: &str) -> Vec<Effect> {
    let Some((gate, reason)) = waiver.split_once(char::is_whitespace) else {
        return invalid_plan_command(state, "Use /plan waive <gate-id> <reason>.");
    };
    let Ok(gate) = gate.parse::<scriberig_core::EntityId>() else {
        return invalid_plan_command(state, "The gate identity is invalid.");
    };
    let reason = reason.trim();
    if reason.is_empty() {
        return invalid_plan_command(state, "A gate waiver requires an explicit reason.");
    }
    let Some((session, plan)) = selected_plan(state) else {
        return invalid_plan_command(state, "Select a durable plan before waiving a gate.");
    };
    vec![Effect::MutatePlanSemantics(
        scriberig_app::PlanSemanticMutationRequest {
            session: session.id,
            plan: plan.id,
            expected_revision: plan.revision,
            mutation: scriberig_app::PlanSemanticMutation::WaiveGate {
                gate,
                reason: reason.to_owned(),
            },
        },
    )]
}

/// Parses a non-empty title and purpose from one work command.
///
/// - `work`: User-authored text in `title: purpose` form.
///
/// Bounded title and non-empty purpose, or `None` for malformed input.
/// # Returns
/// The normalized work fields.
fn parse_work(work: &str) -> Option<(String, String)> {
    let (title, purpose) = work.split_once(':')?;
    let title = title.trim();
    let purpose = purpose.trim();
    if title.is_empty() || purpose.is_empty() {
        return None;
    }
    Some((title.chars().take(80).collect(), purpose.to_owned()))
}

/// Resolves the selected plan against the exact current session projection.
///
/// - `state`: Presentation state holding session and plan identity selection.
///
/// Exact session and plan, when both remain available.
/// # Returns
/// The selected durable entities.
fn selected_plan(
    state: &TuiState,
) -> Option<(&scriberig_session::Session, &scriberig_session::Plan)> {
    let session = state.session_snapshot.as_ref()?;
    let selected = state.plan_cursor.map(|selection| selection.plan);
    let plan = selected
        .and_then(|id| session.plans.iter().find(|plan| plan.id == id))
        .or_else(|| session.plans.last())?;
    Some((session, plan))
}
