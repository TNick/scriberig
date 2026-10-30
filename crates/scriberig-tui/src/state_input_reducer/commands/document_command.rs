//! Validates semantic paragraph targets and routes shared DOCX candidates.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Proposes replacement text for one paragraph in the loaded exact-snapshot tree.
///
/// # Parameters
/// - `state`: Current TUI state with the loaded document and project context.
/// - `arguments`: Paragraph entity ID followed by replacement text.
///
/// # Returns
/// Candidate effect when the exact paragraph is available, or a visible error.
pub(super) fn replace_paragraph(state: &mut TuiState, arguments: &str) -> Vec<Effect> {
    let Some((identity, replacement)) = arguments.split_once(char::is_whitespace) else {
        return usage_error(state);
    };
    let Ok(paragraph) = identity.parse::<scriberig_core::EntityId>() else {
        return usage_error(state);
    };
    let replacement = replacement.trim_start();
    if replacement.is_empty() {
        return usage_error(state);
    }
    let LoadState::Ready(tree) = &state.document_tree else {
        state.conversation_composer.error =
            Some("Load the semantic document before proposing a paragraph edit.".to_owned());
        return vec![Effect::None];
    };
    if !contains_paragraph(&tree.sections, &paragraph) {
        state.conversation_composer.error =
            Some("The paragraph ID is not present in the loaded document.".to_owned());
        return vec![Effect::None];
    }
    let (Ok(project), Ok(snapshot)) = (
        tree.project.parse::<scriberig_core::ProjectId>(),
        tree.snapshot.parse::<scriberig_core::SnapshotId>(),
    ) else {
        state.conversation_composer.error =
            Some("The loaded document has invalid project or snapshot identity.".to_owned());
        return vec![Effect::None];
    };
    let Some(selection) = state.agent_selection.as_ref() else {
        state.conversation_composer.error =
            Some("Document edits require an active project selection.".to_owned());
        return vec![Effect::None];
    };
    if selection.project != Some(project)
        || state.document_snapshot != Some(snapshot)
        || state
            .document_root
            .as_ref()
            .is_none_or(|root| root.to_string() != tree.document)
    {
        state.conversation_composer.error =
            Some("The loaded document no longer matches the selected project snapshot.".to_owned());
        return vec![Effect::None];
    }
    super::super::reduce(
        state,
        Action::DocumentEditRequested(scriberig_app::DocumentParagraphEditQuery {
            project,
            snapshot,
            paragraph,
            replacement: replacement.to_owned(),
        }),
    )
}

/// Proposes replacement text for one cell in the loaded exact-snapshot tree.
///
/// # Parameters
/// - `state`: Current TUI state with the loaded document and project context.
/// - `arguments`: Cell entity ID followed by replacement text.
///
/// # Returns
/// Candidate effect when the exact cell is available, or a visible error.
pub(super) fn replace_table_cell(state: &mut TuiState, arguments: &str) -> Vec<Effect> {
    let Some((identity, replacement)) = arguments.split_once(char::is_whitespace) else {
        return cell_usage_error(state);
    };
    let Ok(cell) = identity.parse::<scriberig_core::EntityId>() else {
        return cell_usage_error(state);
    };
    let replacement = replacement.trim_start();
    if replacement.is_empty() {
        return cell_usage_error(state);
    }
    let LoadState::Ready(tree) = &state.document_tree else {
        state.conversation_composer.error =
            Some("Load the semantic document before proposing a table-cell edit.".to_owned());
        return vec![Effect::None];
    };
    if !contains_cell(&tree.sections, &cell) {
        state.conversation_composer.error =
            Some("The cell ID is not present in the loaded document.".to_owned());
        return vec![Effect::None];
    }
    let (Ok(project), Ok(snapshot)) = (
        tree.project.parse::<scriberig_core::ProjectId>(),
        tree.snapshot.parse::<scriberig_core::SnapshotId>(),
    ) else {
        state.conversation_composer.error =
            Some("The loaded document has invalid project or snapshot identity.".to_owned());
        return vec![Effect::None];
    };
    let Some(selection) = state.agent_selection.as_ref() else {
        state.conversation_composer.error =
            Some("Document edits require an active project selection.".to_owned());
        return vec![Effect::None];
    };
    if selection.project != Some(project)
        || state.document_snapshot != Some(snapshot)
        || state
            .document_root
            .as_ref()
            .is_none_or(|root| root.to_string() != tree.document)
    {
        state.conversation_composer.error =
            Some("The loaded document no longer matches the selected project snapshot.".to_owned());
        return vec![Effect::None];
    }
    super::super::reduce(
        state,
        Action::DocumentTableCellEditRequested(scriberig_app::DocumentTableCellEditQuery {
            project,
            snapshot,
            cell,
            replacement: replacement.to_owned(),
        }),
    )
}

/// Finds a semantic table-cell ID through the loaded section tree.
///
/// # Parameters
/// - `sections`: Ordered semantic sections in the loaded document.
/// - `cell`: Exact semantic cell identity requested for replacement.
///
/// # Returns
/// `true` when the requested ID is a cell in this snapshot.
fn contains_cell(
    sections: &[scriberig_protocol::DocumentSectionView],
    cell: &scriberig_core::EntityId,
) -> bool {
    sections.iter().any(|section| {
        section.blocks.iter().any(|block| match block {
            scriberig_protocol::DocumentBlockView::Paragraph { .. } => false,
            scriberig_protocol::DocumentBlockView::Table { table } => table
                .rows
                .iter()
                .any(|row| row.cells.iter().any(|value| value.id == cell.to_string())),
        })
    })
}

/// Reports the supported exact-target cell replacement syntax.
///
/// # Parameters
/// - `state`: Composer receiving the usage diagnostic.
///
/// # Returns
/// A single no-op effect after presenting the command syntax.
fn cell_usage_error(state: &mut TuiState) -> Vec<Effect> {
    state.conversation_composer.error =
        Some("Usage: /document-cell-replace <cell-id> <replacement text>.".to_owned());
    vec![Effect::None]
}

/// Finds a paragraph by semantic ID through nested tables.
///
/// # Parameters
/// - `sections`: Ordered semantic sections in the loaded document.
/// - `paragraph`: Exact semantic paragraph identity requested for replacement.
///
/// # Returns
/// `true` when the requested ID is a paragraph in this snapshot.
fn contains_paragraph(
    sections: &[scriberig_protocol::DocumentSectionView],
    paragraph: &scriberig_core::EntityId,
) -> bool {
    sections.iter().any(|section| {
        section.blocks.iter().any(|block| match block {
            scriberig_protocol::DocumentBlockView::Paragraph { paragraph: value } => {
                value.id == paragraph.to_string()
            }
            scriberig_protocol::DocumentBlockView::Table { table } => {
                table.rows.iter().any(|row| {
                    row.cells
                        .iter()
                        .any(|cell| contains_blocks(&cell.blocks, paragraph))
                })
            }
        })
    })
}

/// Searches paragraphs inside nested table cells.
///
/// # Parameters
/// - `blocks`: Nested semantic blocks in one table cell.
/// - `paragraph`: Exact semantic paragraph identity requested for replacement.
///
/// # Returns
/// `true` when the target paragraph occurs in these blocks.
fn contains_blocks(
    blocks: &[scriberig_protocol::DocumentBlockView],
    paragraph: &scriberig_core::EntityId,
) -> bool {
    blocks.iter().any(|block| match block {
        scriberig_protocol::DocumentBlockView::Paragraph { paragraph: value } => {
            value.id == paragraph.to_string()
        }
        scriberig_protocol::DocumentBlockView::Table { table } => table.rows.iter().any(|row| {
            row.cells
                .iter()
                .any(|cell| contains_blocks(&cell.blocks, paragraph))
        }),
    })
}

/// Reports the supported exact-target paragraph replacement syntax.
///
/// # Parameters
/// - `state`: Composer receiving the usage diagnostic.
///
/// # Returns
/// A single no-op effect after presenting the command syntax.
fn usage_error(state: &mut TuiState) -> Vec<Effect> {
    state.conversation_composer.error =
        Some("Usage: /document-replace <paragraph-id> <replacement text>.".to_owned());
    vec![Effect::None]
}
