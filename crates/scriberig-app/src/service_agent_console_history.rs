//! Records bounded durable model and thinking selection provenance.

use scriberig_session::{AgentActivityKind, SessionExecutionChange, SessionExecutionSelection};

const MAX_SELECTION_HISTORY: usize = 100;

/// Appends bounded durable provenance for one execution-selection change.
///
/// - `selection`: Updated authoritative session selection.
/// - `kind`: Model or thinking selection category.
/// - `summary`: Safe concise description of the change.
pub(super) fn record_selection_change(
    selection: &mut SessionExecutionSelection,
    kind: AgentActivityKind,
    summary: String,
) {
    selection.history.push(SessionExecutionChange {
        revision: selection.revision,
        kind,
        summary,
        provider_id: selection.provider_id.clone(),
        model_id: selection.model_id.clone(),
        thinking_level: selection.thinking_level.clone(),
    });
    if selection.history.len() > MAX_SELECTION_HISTORY {
        let excess = selection.history.len() - MAX_SELECTION_HISTORY;
        selection.history.drain(..excess);
    }
}
