//! Builds shared plan mutation requests from focused plan commands.

use crate::{effect::Effect, state::TuiState};

use super::invalid_plan_command;

/// Adds next-step or decomposed child work beneath the selected plan node.
///
/// - `state`: Current durable session and stable plan selection.
/// - `work`: Node title and purpose in `title: purpose` form.
/// - `child`: Whether the node is a required child of the selected node.
///
/// A shared plan mutation effect or a local validation message.
/// # Returns
/// Effects for the requested mutation.
pub(super) fn mutate_selected_node(state: &mut TuiState, work: &str, child: bool) -> Vec<Effect> {
    let Some((title, purpose)) = work.split_once(':') else {
        return invalid_plan_command(state, "Use /plan next|child <title>: <purpose>.");
    };
    let title = title.trim();
    let purpose = purpose.trim();
    if title.is_empty() || purpose.is_empty() {
        return invalid_plan_command(state, "Plan node title and purpose must not be empty.");
    }
    let Some(session) = state.session_snapshot.as_ref() else {
        return invalid_plan_command(state, "Load a session before changing a plan.");
    };
    let Some(selection) = state.plan_cursor else {
        return invalid_plan_command(state, "Select a plan node before adding work.");
    };
    let Some(plan) = session.plans.iter().find(|plan| plan.id == selection.plan) else {
        return invalid_plan_command(state, "The selected plan is no longer available.");
    };
    let node = plan_node_draft(title, purpose);
    let mutation = if child {
        let Some(parent) = selection.node else {
            return invalid_plan_command(state, "Select a plan node before adding a child.");
        };
        scriberig_app::PlanMutation::Decompose {
            parent,
            children: vec![node],
            required: true,
        }
    } else {
        scriberig_app::PlanMutation::AddNextStep(node)
    };
    vec![Effect::PlanMutation(plan_mutation_request(
        session.id,
        plan.id,
        plan.revision,
        mutation,
    ))]
}

/// Retries the selected failed plan node through shared application semantics.
///
/// - `state`: Current durable session and stable plan selection.
///
/// A shared retry effect or a local validation message.
/// # Returns
/// Effects for the requested retry.
pub(super) fn retry_selected_node(state: &mut TuiState) -> Vec<Effect> {
    let Some(session) = state.session_snapshot.as_ref() else {
        return invalid_plan_command(state, "Load a session before retrying plan work.");
    };
    let Some(selection) = state.plan_cursor else {
        return invalid_plan_command(state, "Select a failed plan node before retrying it.");
    };
    let Some(node) = selection.node else {
        return invalid_plan_command(state, "Select a failed plan node before retrying it.");
    };
    let Some(plan) = session.plans.iter().find(|plan| plan.id == selection.plan) else {
        return invalid_plan_command(state, "The selected plan is no longer available.");
    };
    vec![Effect::PlanMutation(plan_mutation_request(
        session.id,
        plan.id,
        plan.revision,
        scriberig_app::PlanMutation::RetryFailedNode { node },
    ))]
}

/// Waives a durable gate with an explicit reason through the shared mutation operation.
///
/// - `state`: Current durable session and plan selection.
/// - `waiver`: Gate identity followed by the user-authored reason.
///
/// A shared waiver effect or a local validation message.
/// # Returns
/// Effects for the requested waiver.
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
    let Some(session) = state.session_snapshot.as_ref() else {
        return invalid_plan_command(state, "Load a session before waiving a gate.");
    };
    let Some(selection) = state.plan_cursor else {
        return invalid_plan_command(state, "Select a plan before waiving a gate.");
    };
    let Some(plan) = session.plans.iter().find(|plan| plan.id == selection.plan) else {
        return invalid_plan_command(state, "The selected plan is no longer available.");
    };
    vec![Effect::PlanMutation(plan_mutation_request(
        session.id,
        plan.id,
        plan.revision,
        scriberig_app::PlanMutation::WaiveGate {
            gate,
            reason: reason.to_owned(),
        },
    ))]
}

/// Creates a bounded plan-node draft from explicit user text.
///
/// - `title`: Human-readable node title.
/// - `purpose`: Focused execution instruction.
///
/// The unassigned node draft for the shared plan operation.
/// # Returns
/// The structured node proposal.
fn plan_node_draft(title: &str, purpose: &str) -> scriberig_app::PlanNodeDraft {
    scriberig_app::PlanNodeDraft {
        key: "tui-plan-action".to_owned(),
        parent_index: None,
        title: title.chars().take(80).collect(),
        purpose: purpose.to_owned(),
        expected_result: Some(scriberig_session::ExpectedResult {
            kind: "summary".to_owned(),
            requires_handle: false,
        }),
        semantic_scope: Vec::new(),
        assignment: None,
    }
}

/// Constructs an optimistic typed plan mutation request.
///
/// - `session`: Durable owning session.
/// - `plan`: Stable plan identity.
/// - `expected_revision`: Exact plan revision observed by the TUI.
/// - `mutation`: Typed mutation routed through the application service.
///
/// The shared application request for atomic validation and persistence.
/// # Returns
/// The mutation request.
fn plan_mutation_request(
    session: scriberig_core::EntityId,
    plan: scriberig_core::EntityId,
    expected_revision: u64,
    mutation: scriberig_app::PlanMutation,
) -> scriberig_app::PlanMutationRequest {
    scriberig_app::PlanMutationRequest {
        session,
        plan,
        expected_revision,
        mutation,
    }
}
