//! Reduces bounded streamed assistant output observations.

use super::*;

/// Retains one sanitized live output observation for the session view.
///
/// - `state`: TUI state retaining bounded transient output.
/// - `operation`: Stable shared operation identity.
/// - `text`: Provider output delta or completed text.
/// - `complete`: Whether the observation contains the full output.
///
/// A no-op effect after updating the transient observation list.
/// # Returns
/// The operation result.
pub(in crate::state::state_reducer_tail) fn agent_output_observed(
    state: &mut TuiState,
    operation: &scriberig_core::OperationId,
    text: &str,
    complete: bool,
) -> Vec<Effect> {
    let branch = state
        .agent_handles
        .iter()
        .rev()
        .find(|handle| &handle.operation == operation)
        .and_then(|handle| handle.branch);
    let safe_text = crate::inspector::sanitize(text);
    let observation = state
        .transient_agent_outputs
        .iter_mut()
        .find(|output| &output.operation == operation);
    let new_sequence = state.next_transient_sequence;
    if let Some(observation) = observation {
        if !observation.complete {
            if complete {
                safe_text
                    .chars()
                    .take(8192)
                    .collect::<String>()
                    .clone_into(&mut observation.text);
            } else {
                observation.text.extend(
                    safe_text
                        .chars()
                        .take(8192usize.saturating_sub(observation.text.chars().count())),
                );
            }
            observation.branch = branch.or(observation.branch);
            observation.complete = complete;
        }
    } else {
        state
            .transient_agent_outputs
            .push(crate::state::TransientAgentOutput {
                operation: operation.clone(),
                sequence: new_sequence,
                branch,
                text: safe_text.chars().take(8192).collect(),
                complete,
            });
        state.next_transient_sequence = new_sequence.saturating_add(1);
        if state.transient_agent_outputs.len() > 32 {
            state.transient_agent_outputs.remove(0);
        }
    }
    crate::state::refresh_conversation_projection(state);
    state.shell_message = "agent output streaming".to_owned();
    vec![Effect::None]
}
