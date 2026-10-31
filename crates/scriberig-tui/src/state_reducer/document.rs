//! Reduces exact-snapshot document reads and semantic-part selection.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles document tree requests, results, and stable part selection.
///
/// # Parameters
/// - `state`: Presentation state receiving the document tree and selection.
/// - `action`: Action to classify and reduce.
///
/// # Returns
/// Document service effects when the action belongs to document navigation.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    if let Some(effects) = super::document_edit::reduce(state, action) {
        return Some(effects);
    }
    match action {
        Action::DocumentRequested(request) => {
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.primary_view = crate::selection::PrimaryView::Overview;
            state.artifact_page = LoadState::NotRequested;
            state.spreadsheet_page = LoadState::NotRequested;
            state.spreadsheet_cell = LoadState::NotRequested;
            state.spreadsheet_edit = LoadState::NotRequested;
            state.document_root = Some(request.document);
            state.document_snapshot = Some(request.snapshot);
            state.document_tree = LoadState::Loading { request_id };
            state.document_edit = LoadState::NotRequested;
            state.document_offset = 0;
            Some(vec![Effect::InspectDocument {
                request_id,
                request: *request,
            }])
        }
        Action::DocumentLoaded { request_id, tree } => {
            if matches!(state.document_tree, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.connection = crate::state::ConnectionState::Connected;
                state.stale = false;
                state.document_tree = LoadState::Ready(tree.clone());
                state.document_root = tree.document.parse().ok();
                state.document_snapshot = tree.snapshot.parse().ok();
            }
            Some(vec![Effect::None])
        }
        Action::DocumentFailed {
            request_id,
            category,
            message,
        } => {
            if category == "transport_unavailable" {
                state.connection = crate::state::ConnectionState::Disconnected;
                state.stale = true;
            }
            if matches!(state.document_tree, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.document_tree = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            Some(vec![Effect::None])
        }
        Action::DocumentPartSelected { entity } => {
            let LoadState::Ready(tree) = &state.document_tree else {
                return Some(vec![Effect::None]);
            };
            if !contains_part(&tree.sections, entity) {
                state.conversation_composer.error =
                    Some("That semantic part is not present in the loaded document.".to_owned());
                return Some(vec![Effect::None]);
            }
            let Ok(snapshot) = tree.snapshot.parse() else {
                state.conversation_composer.error =
                    Some("The loaded document snapshot identity is invalid.".to_owned());
                return Some(vec![Effect::None]);
            };
            state.selection = Some(crate::selection::EntitySelection {
                entity: *entity,
                revision: None,
                snapshot: Some(snapshot),
            });
            state.primary_view = crate::selection::PrimaryView::Overview;
            Some(vec![Effect::None])
        }
        Action::DocumentPageSelected { offset } => {
            if matches!(state.document_tree, LoadState::Ready(_)) {
                state.document_offset = *offset;
                state.primary_view = crate::selection::PrimaryView::Overview;
            }
            Some(vec![Effect::None])
        }
        _ => None,
    }
}

/// Checks whether a semantic part occurs in a loaded section tree.
///
/// # Parameters
/// - `sections`: Ordered sections returned by the document service.
/// - `target`: Exact entity identity selected by the user.
///
/// # Returns
/// `true` when the target identifies a paragraph, table, row, or cell.
fn contains_part(
    sections: &[scriberig_protocol::DocumentSectionView],
    target: &scriberig_core::EntityId,
) -> bool {
    sections
        .iter()
        .any(|section| section.id == target.to_string() || contains_blocks(&section.blocks, target))
}

/// Checks recursively nested paragraphs and table parts for one stable identity.
///
/// # Parameters
/// - `blocks`: Semantic document blocks to search.
/// - `target`: Exact entity identity selected by the user.
///
/// # Returns
/// `true` when the target is a paragraph, table, row, or cell in the blocks.
fn contains_blocks(
    blocks: &[scriberig_protocol::DocumentBlockView],
    target: &scriberig_core::EntityId,
) -> bool {
    blocks.iter().any(|block| match block {
        scriberig_protocol::DocumentBlockView::Paragraph { paragraph } => {
            paragraph.id == target.to_string()
        }
        scriberig_protocol::DocumentBlockView::Table { table } => {
            table.id == target.to_string()
                || table.rows.iter().any(|row| {
                    row.id == target.to_string()
                        || row.cells.iter().any(|cell| {
                            cell.id == target.to_string() || contains_blocks(&cell.blocks, target)
                        })
                })
        }
    })
}
