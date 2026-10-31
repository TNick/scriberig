//! Converts spreadsheet wire values to shared semantic values.

use scriberig_domain_spreadsheet::{CellValue, Formula};
use scriberig_protocol::SpreadsheetCellValue;

/// Converts a protocol value into the domain-owned spreadsheet schema.
///
/// - `value`: Typed protocol value, including nested cached formula results.
///
/// # Returns
/// Semantic cell value without wire-specific types.
pub(super) fn cell_value(value: SpreadsheetCellValue) -> CellValue {
    match value {
        SpreadsheetCellValue::Blank => CellValue::Blank,
        SpreadsheetCellValue::Text(value) => CellValue::Text(value),
        SpreadsheetCellValue::Number(value) => CellValue::Number(value),
        SpreadsheetCellValue::Boolean(value) => CellValue::Boolean(value),
        SpreadsheetCellValue::Error(value) => CellValue::Error(value),
        SpreadsheetCellValue::Formula {
            expression,
            cached_value,
        } => CellValue::Formula(Formula {
            expression,
            cached_value: cached_value.map(|value| Box::new(cell_value(*value))),
        }),
    }
}
