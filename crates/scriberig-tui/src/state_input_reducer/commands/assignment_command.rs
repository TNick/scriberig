//! Resolves semantic plan assignment targets for the command reducer.

use super::*;

/// Selects the exact plan node whose execution assignment should be edited.
///
/// # Parameters
/// - `state`: Current reducer state and plan selection.
///
/// # Returns
/// The model-library read effect, or no effect when the library is already loaded.
pub(super) fn assign(state: &mut TuiState) -> Vec<Effect> {
    let node = if state.primary_view == crate::selection::PrimaryView::Plan {
        state.plan_cursor.and_then(|selection| selection.node)
    } else {
        match state.conversation_cursor {
            Some(crate::view::conversation::ConversationSource::PlanNode(node)) => Some(node),
            _ => None,
        }
    };
    let Some(node) = node else {
        state.conversation_composer.error =
            Some("Select a plan node before using /assign.".to_owned());
        return vec![Effect::None];
    };
    let Some(plan) = state.session_snapshot.as_ref().and_then(|session| {
        session
            .plans
            .iter()
            .find(|plan| plan.nodes.iter().any(|candidate| candidate.id == node))
    }) else {
        state.conversation_composer.error =
            Some("The selected plan node is no longer available.".to_owned());
        return vec![Effect::None];
    };
    state.plan_assignment_target = Some(crate::state::PlanAssignmentTarget {
        plan: plan.id,
        node,
        expected_revision: plan.revision,
    });
    if matches!(state.model_library, crate::inspector::LoadState::Ready(_)) {
        return vec![Effect::None];
    }
    if matches!(
        state.model_library,
        crate::inspector::LoadState::Loading { .. }
    ) {
        return vec![Effect::None];
    }
    let request_id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    state.model_library = crate::inspector::LoadState::Loading { request_id };
    vec![Effect::ReadModelLibrary { request_id }]
}
