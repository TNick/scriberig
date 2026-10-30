//! Writes and validates semantic edits in retained XLSX packages.

use std::collections::BTreeSet;
use std::io::Cursor;

use scriberig_domain_spreadsheet::CellValue;

use super::{invalid_cell, map_error};
use crate::XlsxAdapterError;

/// Describes one sparse worksheet value edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XlsxCellEdit {
    /// One-based worksheet row.
    pub row: u32,
    /// One-based worksheet column.
    pub column: u32,
    /// Typed semantic value to write.
    pub value: CellValue,
}

/// Maximum cell values accepted by one retained-package edit.
pub const MAX_XLSX_CELL_EDITS: usize = 500;

/// Applies one semantic cell value and returns package-preserving XLSX bytes.
///
/// - bytes: Exact current XLSX package bytes.
/// - worksheet: Authored worksheet name.
/// - row: One-based row coordinate.
/// - column: One-based column coordinate.
/// - value: Semantic cell value to write.
///
/// # Parameters
/// - `bytes`: Exact current XLSX package bytes.
/// - `worksheet`: Authored worksheet name.
/// - `row`: One-based row coordinate.
/// - `column`: One-based column coordinate.
/// - `value`: Semantic cell value to write.
///
/// # Returns
/// Validated XLSX package bytes retaining untouched source parts.
///
/// # Errors
/// Returns an error when the package is read-only or the semantic value cannot
/// be represented by the retained-package writer.
pub fn edit_cell(
    bytes: &[u8],
    worksheet: &str,
    row: u32,
    column: u32,
    value: &CellValue,
) -> Result<Vec<u8>, XlsxAdapterError> {
    edit_cells(
        bytes,
        worksheet,
        &[XlsxCellEdit {
            row,
            column,
            value: value.clone(),
        }],
    )
}

/// Applies a bounded set of semantic cell values in one workbook rewrite.
///
/// # Parameters
/// - `bytes`: Exact current XLSX package bytes.
/// - `worksheet`: Authored worksheet name.
/// - `edits`: One through five hundred distinct cell replacements.
///
/// # Returns
/// Validated XLSX package bytes retaining untouched source parts.
///
/// # Errors
/// Returns an error for an empty/oversized set, duplicate or invalid coordinates,
/// a missing worksheet, read-only input, or values the package writer cannot represent.
pub fn edit_cells(
    bytes: &[u8],
    worksheet: &str,
    edits: &[XlsxCellEdit],
) -> Result<Vec<u8>, XlsxAdapterError> {
    if edits.is_empty() || edits.len() > MAX_XLSX_CELL_EDITS {
        return Err(invalid_cell(
            "cell edit count must be between one and five hundred",
        ));
    }
    let mut spreadsheet = rxls::Spreadsheet::open(bytes).map_err(|error| map_error(&error))?;
    if !spreadsheet
        .workbook()
        .sheets
        .iter()
        .any(|sheet| sheet.is_worksheet && sheet.name == worksheet)
    {
        return Err(XlsxAdapterError {
            code: "xlsx_worksheet_not_found",
            message: format!("worksheet {worksheet:?} does not exist in the source workbook"),
        });
    }
    let mut coordinates = BTreeSet::new();
    for edit in edits {
        if edit.row == 0 || edit.column == 0 || edit.column > 16_384 {
            return Err(invalid_cell("cell coordinates are outside the XLSX grid"));
        }
        if !coordinates.insert((edit.row, edit.column)) {
            return Err(invalid_cell("cell edit set contains duplicate coordinates"));
        }
        let rxls_row = edit.row - 1;
        let rxls_column = u16::try_from(edit.column - 1)
            .map_err(|_| invalid_cell("column is outside the XLSX grid"))?;
        match &edit.value {
            CellValue::Blank => spreadsheet.clear_cell_value(worksheet, rxls_row, rxls_column),
            CellValue::Text(value) => spreadsheet.set_cell_value(
                worksheet,
                rxls_row,
                rxls_column,
                rxls::Cell::Text(value.clone()),
            ),
            CellValue::Number(value) => {
                let number = value
                    .parse::<f64>()
                    .map_err(|_| invalid_cell("numeric cell value is not a valid number"))?;
                spreadsheet.set_cell_value(
                    worksheet,
                    rxls_row,
                    rxls_column,
                    rxls::Cell::Number(number),
                )
            }
            CellValue::Boolean(value) => spreadsheet.set_cell_value(
                worksheet,
                rxls_row,
                rxls_column,
                rxls::Cell::Bool(*value),
            ),
            CellValue::Error(value) => spreadsheet.set_cell_value(
                worksheet,
                rxls_row,
                rxls_column,
                rxls::Cell::Error(value.clone()),
            ),
            CellValue::Formula(formula) => {
                let cached = formula
                    .cached_value
                    .as_deref()
                    .map(to_rxls_cell)
                    .transpose()?
                    .unwrap_or_else(|| rxls::Cell::Text(String::new()));
                spreadsheet.set_cell_formula(
                    worksheet,
                    rxls_row,
                    rxls_column,
                    formula.expression.trim_start_matches('='),
                    cached,
                )
            }
        }
        .map_err(|error| map_error(&error))?;
    }
    spreadsheet.save().map_err(|error| map_error(&error))
}

/// Renames one worksheet using the retained-package reference updater.
///
/// The pinned package writer rewrites supported formula, defined-name, chart,
/// hyperlink, and worksheet-source references atomically. Unsupported touched
/// package structures fail the operation instead of producing partial output.
///
/// # Parameters
/// - `bytes`: Exact current XLSX package bytes.
/// - `old_name`: Exact current worksheet name.
/// - `new_name`: Requested worksheet name.
///
/// # Returns
/// Updated XLSX bytes with supported dependent references rewritten.
///
/// # Errors
/// Returns an error when the package is read-only, the target is missing, the
/// new name is invalid or duplicated, or a dependent package feature is unsupported.
pub fn rename_sheet(
    bytes: &[u8],
    old_name: &str,
    new_name: &str,
) -> Result<Vec<u8>, XlsxAdapterError> {
    let mut spreadsheet = rxls::Spreadsheet::open(bytes).map_err(|error| map_error(&error))?;
    spreadsheet
        .rename_sheet(old_name, new_name)
        .map_err(|error| map_error(&error))?;
    spreadsheet.save().map_err(|error| map_error(&error))
}

/// Reopens an XLSX package and verifies it remains editable after staging.
///
/// # Parameters
/// - `bytes`: Exact staged package bytes.
///
/// # Returns
/// Unit when the package reopens with retained-package edit capability.
///
/// # Errors
/// Returns an error when parsing fails or the package must be treated read-only.
pub fn validate_package(bytes: &[u8]) -> Result<(), XlsxAdapterError> {
    validate_zip_structure(bytes)?;
    let spreadsheet = rxls::Spreadsheet::open(bytes).map_err(|error| map_error(&error))?;
    match spreadsheet.edit_capability() {
        rxls::EditCapability::ReadWrite => Ok(()),
        rxls::EditCapability::ReadOnly(reason) => Err(XlsxAdapterError {
            code: "xlsx_package_read_only",
            message: format!("staged XLSX package is read-only: {reason:?}"),
        }),
    }
}

/// Independently checks the XLSX ZIP directory, required roots, and part checksums.
///
/// # Parameters
/// - `bytes`: Exact staged package bytes.
///
/// # Returns
/// Unit when all archive entries are readable and required workbook parts exist.
///
/// # Errors
/// Returns a structured error for malformed ZIP data, unreadable parts, or missing roots.
fn validate_zip_structure(bytes: &[u8]) -> Result<(), XlsxAdapterError> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| XlsxAdapterError {
            code: "xlsx_zip_invalid",
            message: format!("staged XLSX ZIP directory is invalid: {error}"),
        })?;
    for required in ["[Content_Types].xml", "_rels/.rels", "xl/workbook.xml"] {
        if archive.by_name(required).is_err() {
            return Err(XlsxAdapterError {
                code: "xlsx_zip_required_part_missing",
                message: format!("staged XLSX is missing required package part {required}"),
            });
        }
    }
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).map_err(|error| XlsxAdapterError {
            code: "xlsx_zip_part_invalid",
            message: format!("staged XLSX part directory entry is invalid: {error}"),
        })?;
        std::io::copy(&mut member, &mut std::io::sink()).map_err(|error| XlsxAdapterError {
            code: "xlsx_zip_part_invalid",
            message: format!("staged XLSX package part failed checksum/read validation: {error}"),
        })?;
    }
    Ok(())
}

/// Converts one shared scalar value to the retained-package cell vocabulary.
///
/// # Parameters
/// - `value`: Shared cell value to represent.
///
/// # Returns
/// A parser-native scalar cell.
///
/// # Errors
/// Returns an error when the value is blank or formula-valued.
fn to_rxls_cell(value: &CellValue) -> Result<rxls::Cell, XlsxAdapterError> {
    match value {
        CellValue::Text(value) => Ok(rxls::Cell::Text(value.clone())),
        CellValue::Number(value) => value
            .parse::<f64>()
            .map(rxls::Cell::Number)
            .map_err(|_| invalid_cell("formula cache is not a valid number")),
        CellValue::Boolean(value) => Ok(rxls::Cell::Bool(*value)),
        CellValue::Error(value) => Ok(rxls::Cell::Error(value.clone())),
        CellValue::Blank | CellValue::Formula(_) => Err(invalid_cell(
            "formula cache must be a scalar nonblank value",
        )),
    }
}
