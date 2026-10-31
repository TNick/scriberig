//! Renders bounded semantic spreadsheet pages through the upstream Markdown component.

use super::super::common::{SemanticRole, append_status, append_value, render_load_state};
use crate::inspector::LoadState;

/// Renders typed sparse cell values and exact snapshot metadata.
///
/// # Parameters
/// - `state`: Spreadsheet page load state.
///
/// # Returns
/// Markdown content suitable for the upstream RPI terminal renderer.
pub(super) fn render(state: &LoadState<scriberig_protocol::SpreadsheetCellsResponse>) -> String {
    render_load_state("Spreadsheet range", state, |page, output| {
        append_value(output, &format!("Project: {}", page.project));
        append_value(output, &format!("Snapshot: {}", page.snapshot));
        append_value(output, &format!("Worksheet: {}", page.worksheet));
        if let Some(reason) = &page.artifact_diagnostic {
            append_status(output, "EDITING RESTRICTED", reason, SemanticRole::Warning);
        }
        append_value(
            output,
            &format!("{} of {} matching cells", page.cells.len(), page.total),
        );
        for cell in page.cells.iter().take(40) {
            let address = format!("{}{}", column_label(cell.column), cell.row);
            match &cell.value {
                scriberig_protocol::SpreadsheetCellValue::Formula {
                    expression,
                    cached_value,
                } => {
                    append_value(output, &format!("## {address} · {}", cell.id));
                    append_value(output, &format!("Formula: {expression}"));
                    if let Some(value) = cached_value {
                        append_value(output, &format!("Cached result: {}", format_value(value)));
                    }
                }
                value => append_value(
                    output,
                    &format!("{address} · {} · {}", cell.id, format_value(value)),
                ),
            }
        }
        if page.has_more {
            append_value(
                output,
                "More cells are available. Request another page with the same range and an offset 40 greater than this page's offset.",
            );
        }
    })
}

/// Renders one exact coordinate inspection, including explicit sparse blanks.
///
/// # Parameters
/// - `state`: Coordinate inspection load state.
///
/// # Returns
/// Markdown with the semantic cell identity and formula/value details.
pub(super) fn render_cell(
    state: &LoadState<scriberig_protocol::SpreadsheetCellInspectResponse>,
) -> String {
    render_load_state("Spreadsheet cell", state, |inspection, output| {
        append_value(output, &format!("Project: {}", inspection.project));
        append_value(output, &format!("Snapshot: {}", inspection.snapshot));
        append_value(output, &format!("Worksheet: {}", inspection.worksheet));
        append_value(
            output,
            &format!(
                "Coordinate: {}{}",
                column_label(inspection.column),
                inspection.row
            ),
        );
        if let Some(cell) = &inspection.cell {
            append_value(output, &format!("Semantic cell: {}", cell.id));
            match &cell.value {
                scriberig_protocol::SpreadsheetCellValue::Formula {
                    expression,
                    cached_value,
                } => {
                    append_value(output, &format!("Formula: {expression}"));
                    if let Some(value) = cached_value {
                        append_value(output, &format!("Cached result: {}", format_value(value)));
                    }
                }
                value => append_value(output, &format!("Value: {}", format_value(value))),
            }
        } else {
            append_value(
                output,
                "This coordinate is a sparse blank; no semantic cell entity exists.",
            );
        }
    })
}

/// Formats a typed authored value without presenting formulas' cached results as authoritative.
///
/// # Parameters
/// - `value`: Protocol cell value.
///
/// # Returns
/// Compact terminal-safe authored value text.
fn format_value(value: &scriberig_protocol::SpreadsheetCellValue) -> String {
    use scriberig_protocol::SpreadsheetCellValue as Value;
    match value {
        Value::Blank => "(blank)".to_owned(),
        Value::Text(value) | Value::Number(value) | Value::Error(value) => value.clone(),
        Value::Boolean(value) => value.to_string(),
        Value::Formula { expression, .. } => expression.clone(),
    }
}

/// Converts a one-based column index to a spreadsheet label.
///
/// # Parameters
/// - `column`: One-based column coordinate.
///
/// # Returns
/// Spreadsheet letters such as `A` or `AA`.
fn column_label(column: u32) -> String {
    let mut value = column;
    let mut label = String::new();
    while value > 0 {
        value -= 1;
        label.insert(
            0,
            char::from_u32(u32::from(b'A') + value % 26).unwrap_or('?'),
        );
        value /= 26;
    }
    label
}
