//! Builds safe details for a selected `ScribeRig` semantic row.

use crate::state::TuiState;

#[expect(
    clippy::too_many_lines,
    reason = "This maps selected ScribeRig semantic row kinds to their exact safe detail projections."
)]
/// Creates details for the currently selected stable semantic row.
///
/// - `state`: Current `ScribeRig` view and cursor.
///
/// Safe view context and selected-row details for upstream Markdown.
/// # Returns
/// Sanitized Markdown details for the current selection.
pub(super) fn selection_details(state: &TuiState) -> String {
    let mut output = String::new();
    match state.primary_view {
        crate::selection::PrimaryView::Explorer => {
            output.push_str("# Semantic explorer\n\n");
            if let Some(root) = &state.explorer.root {
                super::super::common::append_value(
                    &mut output,
                    &format!("Root: {}", root.selection.entity),
                );
            }
            let row = state
                .cursor
                .checked_sub(usize::from(state.explorer.root.is_some()))
                .and_then(|index| state.explorer.rows.get(index));
            if let Some(row) = row {
                super::super::common::append_value(
                    &mut output,
                    &format!("Entity: {}", row.selection.entity),
                );
                super::super::common::append_value(&mut output, &format!("Label: {}", row.label));
                super::super::common::append_value(
                    &mut output,
                    &format!("Kind: {}", row.kind.as_deref().unwrap_or("semantic entity")),
                );
                append_snapshot(&mut output, row.selection.snapshot);
            } else if let Some(root) = &state.explorer.root {
                append_snapshot(&mut output, root.selection.snapshot);
            }
            if state.explorer.next.is_some() {
                super::super::common::append_value(
                    &mut output,
                    "More semantic results are available.",
                );
            }
        }
        crate::selection::PrimaryView::History => {
            if let crate::inspector::LoadState::Ready(model) = &state.history {
                super::super::common::append_value(
                    &mut output,
                    &format!("Snapshot: {}", model.snapshot),
                );
                if let Some(row) = model.rows.get(state.cursor) {
                    super::super::common::append_value(
                        &mut output,
                        &format!("Migration: {}", row.id),
                    );
                    super::super::common::append_value(&mut output, &format!("Base: {}", row.base));
                    super::super::common::append_value(
                        &mut output,
                        &format!("Result: {}", row.result),
                    );
                    for (entity, revision) in row.entities.iter().take(64) {
                        super::super::common::append_value(
                            &mut output,
                            &format!(
                                "{} · revision {}",
                                entity,
                                revision.map_or_else(|| "none".to_owned(), |id| id.to_string())
                            ),
                        );
                    }
                }
            }
        }
        crate::selection::PrimaryView::Find => {
            if let crate::inspector::LoadState::Ready(model) = &state.find {
                super::super::common::append_value(&mut output, &format!("Query: {}", model.query));
                if let Some(row) = model.rows.get(state.cursor) {
                    super::super::common::append_value(
                        &mut output,
                        &format!("Entity: {}", row.entity),
                    );
                    super::super::common::append_value(
                        &mut output,
                        &format!("Type: {}", row.entity_type.as_deref().unwrap_or("entity")),
                    );
                    super::super::common::append_value(
                        &mut output,
                        &format!("Snapshot: {}", row.snapshot),
                    );
                    if let Some(preview) = &row.preview {
                        super::super::common::append_value(&mut output, preview);
                    }
                }
            }
        }
        crate::selection::PrimaryView::Diff => {
            if let crate::inspector::LoadState::Ready(model) = &state.diff {
                super::super::common::append_value(
                    &mut output,
                    &format!("Base snapshot: {}", model.left),
                );
                super::super::common::append_value(
                    &mut output,
                    &format!("Target snapshot: {}", model.right),
                );
                if let Some(row) = model.rows.get(state.cursor) {
                    super::super::common::append_value(
                        &mut output,
                        &format!("Entity: {}", row.entity),
                    );
                    super::super::common::append_value(
                        &mut output,
                        &format!(
                            "Before: {}",
                            row.before
                                .map_or_else(|| "none".to_owned(), |id| id.to_string())
                        ),
                    );
                    super::super::common::append_value(
                        &mut output,
                        &format!(
                            "After: {}",
                            row.after
                                .map_or_else(|| "none".to_owned(), |id| id.to_string())
                        ),
                    );
                }
            }
        }
        crate::selection::PrimaryView::Operations => {
            if let Some(operation) = state.operations.get(state.cursor) {
                super::super::common::append_value(
                    &mut output,
                    &format!("Operation: {}", operation.id),
                );
                super::super::common::append_value(
                    &mut output,
                    &format!("Kind: {} · {:?}", operation.kind, operation.status),
                );
                if let Some(progress) = &operation.progress {
                    super::super::common::append_value(&mut output, progress);
                }
                if let Some(error) = &operation.error {
                    super::super::common::append_value(&mut output, error);
                }
                for handle in operation.result_handles.iter().take(32) {
                    super::super::common::append_value(&mut output, &format!("Result: {handle}"));
                }
            }
            if let Some((category, message)) = &state.operation_error {
                super::super::common::append_value(&mut output, &format!("{category}: {message}"));
            }
        }
        _ => {}
    }
    output
}

/// Appends an exact snapshot reference when present.
///
/// - `output`: Destination Markdown detail text.
/// - `snapshot`: Snapshot context attached to the semantic selection.
fn append_snapshot(output: &mut String, snapshot: Option<scriberig_core::SnapshotId>) {
    super::super::common::append_value(
        output,
        &format!(
            "Snapshot: {}",
            snapshot.map_or_else(|| "none".to_owned(), |id| id.to_string())
        ),
    );
}
