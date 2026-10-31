//! Formats `ScribeRig`'s durable session and operation read models.

use super::common::{MAX_ROWS, SemanticRole, append_status, append_value, render_load_state};
use crate::state::TuiState;

#[cfg(test)]
#[path = "runtime_views_tests.rs"]
mod tests;

/// Renders durable session rows and model assignment labels.
///
/// - `state`: Session tree projection built from the canonical `ScribeRig` session.
///
/// # Returns
/// Stable session row identities and their visible labels.
pub(super) fn session(state: &TuiState) -> String {
    render_load_state("Session tree", &state.session, |model, output| {
        append_value(
            output,
            &format!(
                "Session execution: {} / {}",
                model
                    .execution
                    .provider_id
                    .as_deref()
                    .unwrap_or("unassigned"),
                model.execution.model_id.as_deref().unwrap_or("unassigned")
            ),
        );
        for (index, row) in model.rows.iter().take(MAX_ROWS).enumerate() {
            append_value(
                output,
                &format!(
                    "{}{}{} · {:?} · {}",
                    if index == state.cursor { "› " } else { "  " },
                    "  ".repeat(row.depth.min(8)),
                    row.label,
                    row.kind,
                    row.reference
                ),
            );
        }
    })
}

/// Renders shared operation identities and their lifecycle state.
///
/// - `state`: Operation observations retained by the `ScribeRig` TUI.
///
/// # Returns
/// Operation status and bounded result handles.
pub(super) fn operations(state: &TuiState) -> String {
    let mut output = String::from("# Operations\n\n");
    if state.operations.is_empty() && state.operation_error.is_none() {
        append_value(&mut output, "No observed operations.");
    }
    for (index, operation) in state.operations.iter().take(MAX_ROWS).enumerate() {
        append_value(
            &mut output,
            &format!(
                "{}{} · {}",
                if index == state.cursor { "› " } else { "  " },
                operation.id,
                operation.kind
            ),
        );
        let role = match operation.status {
            scriberig_app::OperationStatus::Completed => SemanticRole::Success,
            scriberig_app::OperationStatus::Failed => SemanticRole::Error,
            scriberig_app::OperationStatus::Cancelled => SemanticRole::Warning,
            scriberig_app::OperationStatus::Queued | scriberig_app::OperationStatus::Running => {
                SemanticRole::Neutral
            }
        };
        append_status(
            &mut output,
            "Status",
            &format!("{:?}", operation.status),
            role,
        );
        if let Some(progress) = &operation.progress {
            append_value(&mut output, progress);
        }
        if let Some(error) = &operation.error {
            append_status(&mut output, "Error", error, SemanticRole::Error);
        }
        for handle in operation.result_handles.iter().take(32) {
            append_value(&mut output, &format!("Result: {handle}"));
        }
    }
    if let Some((category, message)) = &state.operation_error {
        append_status(&mut output, category, message, SemanticRole::Error);
    }
    output
}
