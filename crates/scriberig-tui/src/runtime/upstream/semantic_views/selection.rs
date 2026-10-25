//! Projects stable flat semantic rows into the upstream `SelectList` component.

use rpi_tui::{SelectItem, SelectList};

use crate::state::TuiState;

#[path = "selection/details.rs"]
mod details;
#[cfg(test)]
#[path = "selection/selection_tests.rs"]
mod tests;

/// Carries an existing reducer row identity into an upstream selectable list.
pub(super) struct SelectableRow {
    /// Stable identity used to map upstream selection back to the projection.
    pub(super) id: String,
    /// Stable semantic identity retained by the `ScribeRig` reducer.
    pub(super) identity: crate::selection::SemanticRowIdentity,
    /// Bounded terminal-safe visible label.
    label: String,
    /// Bounded terminal-safe secondary description.
    description: String,
}

/// Reports whether the current primary view has a stable flat row projection.
///
/// - `state`: Current `ScribeRig` primary-view selection.
///
/// `true` for views whose cursor reducer already addresses projected rows.
/// # Returns
/// Whether the primary view has a stable flat selectable projection.
pub(super) fn supports(state: &TuiState) -> bool {
    matches!(
        state.primary_view,
        crate::selection::PrimaryView::Explorer
            | crate::selection::PrimaryView::History
            | crate::selection::PrimaryView::Find
            | crate::selection::PrimaryView::Diff
            | crate::selection::PrimaryView::Operations
    )
}

/// Builds an upstream list and its supplementary semantic details.
///
/// - `state`: Current view data and reducer-owned cursor.
///
/// The upstream list and safe details for a ready list-backed view.
/// # Returns
/// List and details when the active semantic projection is ready.
pub(super) fn semantic_list(state: &TuiState) -> Option<(SelectList, String)> {
    let (list, rows) = list_from_state(state)?;
    if rows.is_empty() {
        return None;
    }
    Some((list, details::selection_details(state)))
}

/// Builds a selectable list from the active flat semantic projection.
///
/// - `state`: Current view data and reducer-owned cursor.
///
/// The upstream list and row identity sequence when the view is ready.
/// # Returns
/// Stable selectable rows when the active view is list-backed and ready.
pub(super) fn list_from_state(state: &TuiState) -> Option<(SelectList, Vec<SelectableRow>)> {
    let rows = rows_for_state(state)?;
    let items = rows
        .iter()
        .map(|row| {
            let search_text = format!("{} {}", row.label, row.description);
            SelectItem::new(&row.id, &row.label)
                .with_description(&row.description)
                .with_search_text(&search_text)
        })
        .collect();
    let list = SelectList::new(items, super::common::MAX_ROWS.min(16));
    let selected = state
        .semantic_row_cursor
        .as_ref()
        .and_then(|identity| rows.iter().position(|row| &row.identity == identity))
        .unwrap_or_else(|| state.cursor.min(rows.len().saturating_sub(1)));
    list.set_selected_index(selected);
    Some((list, rows))
}

#[expect(
    clippy::too_many_lines,
    reason = "Each primary semantic view has a distinct ScribeRig read-model projection."
)]
/// Creates safe stable rows for supported semantic views.
///
/// - `state`: Current `ScribeRig` view and read models.
///
/// Reducer-order rows with stable IDs, or `None` until the model is ready.
/// # Returns
/// Stable row identities in reducer order when data is available.
fn rows_for_state(state: &TuiState) -> Option<Vec<SelectableRow>> {
    match state.primary_view {
        crate::selection::PrimaryView::Explorer => {
            let mut rows = Vec::new();
            if let Some(root) = &state.explorer.root {
                rows.push(SelectableRow {
                    id: explorer_id(root.selection, true),
                    identity: explorer_identity(root.selection, true),
                    label: root.label.clone(),
                    description: format!(
                        "Root · {} · snapshot {}",
                        root.selection.entity,
                        root.selection
                            .snapshot
                            .map_or_else(|| "none".to_owned(), |id| id.to_string())
                    ),
                });
            }
            rows.extend(state.explorer.rows.iter().map(|row| SelectableRow {
                id: explorer_id(row.selection, false),
                identity: explorer_identity(row.selection, false),
                label: row.label.clone(),
                description: format!(
                            "{} · {} · snapshot {}",
                            row.kind.as_deref().unwrap_or("semantic entity"),
                            row.selection.entity,
                            row.selection
                                .snapshot
                                .map_or_else(|| "none".to_owned(), |id| id.to_string())
                        ),
            }));
            Some(rows)
        }
        crate::selection::PrimaryView::History => {
            let crate::inspector::LoadState::Ready(model) = &state.history else {
                return None;
            };
            Some(
                model
                    .rows
                    .iter()
                    .map(|row| SelectableRow {
                        id: format!("history:{}", row.id),
                        identity: crate::selection::SemanticRowIdentity::History(row.id.clone()),
                        label: row.id.clone(),
                        description: format!("{} → {}", row.base, row.result),
                    })
                    .collect(),
            )
        }
        crate::selection::PrimaryView::Find => {
            let crate::inspector::LoadState::Ready(model) = &state.find else {
                return None;
            };
            Some(
                model
                    .rows
                    .iter()
                    .map(|row| SelectableRow {
                        id: format!("find:{}:{}", row.entity, row.snapshot),
                        identity: crate::selection::SemanticRowIdentity::Find(
                            row.entity,
                            row.snapshot,
                        ),
                        label: row.entity.to_string(),
                        description: format!(
                            "{} · snapshot {} · {}",
                            row.entity_type.as_deref().unwrap_or("entity"),
                            row.snapshot,
                            row.preview.as_deref().unwrap_or_default()
                        ),
                    })
                    .collect(),
            )
        }
        crate::selection::PrimaryView::Diff => {
            let crate::inspector::LoadState::Ready(model) = &state.diff else {
                return None;
            };
            Some(
                model
                    .rows
                    .iter()
                    .map(|row| SelectableRow {
                        id: format!("diff:{}:{}:{}", row.entity, model.left, model.right),
                        identity: crate::selection::SemanticRowIdentity::Diff(
                            row.entity,
                            model.left,
                            model.right,
                        ),
                        label: row.entity.to_string(),
                        description: format!(
                            "{} → {}",
                            row.before
                                .map_or_else(|| "none".to_owned(), |id| id.to_string()),
                            row.after
                                .map_or_else(|| "none".to_owned(), |id| id.to_string())
                        ),
                    })
                    .collect(),
            )
        }
        crate::selection::PrimaryView::Operations => Some(
            state
                .operations
                .iter()
                .map(|operation| SelectableRow {
                    id: format!("operation:{}", operation.id),
                    identity: crate::selection::SemanticRowIdentity::Operation(
                        operation.id.clone(),
                    ),
                    label: format!("{} · {:?}", operation.kind, operation.status),
                    description: operation.id.to_string(),
                })
                .collect(),
        ),
        _ => None,
    }
    .map(|rows| {
        rows.into_iter()
            .map(|row| SelectableRow {
                id: row.id,
                identity: row.identity,
                label: bounded(&row.label),
                description: bounded(&row.description),
            })
            .collect()
    })
}

/// Converts an explorer selection into its typed stable row identity.
///
/// - `selection`: Exact semantic selection represented by the row.
/// - `root`: Whether the row is the explorer root.
///
/// The stable identity used across projection refreshes.
/// # Returns
/// Typed identity for the selected explorer row.
fn explorer_identity(
    selection: crate::selection::EntitySelection,
    root: bool,
) -> crate::selection::SemanticRowIdentity {
    crate::selection::SemanticRowIdentity::Explorer {
        entity: selection.entity,
        revision: selection.revision,
        snapshot: selection.snapshot,
        root,
    }
}

/// Converts an explorer selection into its upstream stable row key.
///
/// - `selection`: Exact semantic selection represented by the row.
/// - `root`: Whether the row is the explorer root.
///
/// A namespaced upstream selection key.
/// # Returns
/// Stable upstream list identity for the explorer row.
fn explorer_id(selection: crate::selection::EntitySelection, root: bool) -> String {
    let revision = selection
        .revision
        .map_or_else(|| "none".to_owned(), |id| id.to_string());
    let snapshot = selection
        .snapshot
        .map_or_else(|| "none".to_owned(), |id| id.to_string());
    format!(
        "explorer:{}:{}:{}:{}",
        if root { "root" } else { "entity" },
        selection.entity,
        revision,
        snapshot
    )
}

/// Reconciles the semantic-view cursor after a model or row refresh.
///
/// - `state`: TUI state receiving the refreshed list identity and cursor.
pub fn reconcile_cursor(state: &mut TuiState) {
    let Some(rows) = rows_for_state(state) else {
        return;
    };
    if rows.is_empty() {
        state.cursor = 0;
        state.semantic_row_cursor = None;
        return;
    }
    let index = state
        .semantic_row_cursor
        .as_ref()
        .and_then(|identity| rows.iter().position(|row| &row.identity == identity))
        .unwrap_or_else(|| state.cursor.min(rows.len() - 1));
    state.cursor = index;
    state.semantic_row_cursor = rows.get(index).map(|row| row.identity.clone());
}

/// Updates stable selection after reducer-driven movement.
///
/// - `state`: TUI state receiving the current row identity.
pub fn retain_cursor_identity(state: &mut TuiState) {
    state.semantic_row_cursor = rows_for_state(state)
        .and_then(|rows| rows.get(state.cursor).map(|row| row.identity.clone()));
}

/// Sanitizes and bounds a label before it reaches upstream `SelectList`.
///
/// - `value`: Projected semantic label or description.
///
/// Terminal-safe text limited to 240 characters.
/// # Returns
/// Sanitized and bounded semantic text.
fn bounded(value: &str) -> String {
    crate::inspector::sanitize(value)
        .chars()
        .take(240)
        .collect()
}
