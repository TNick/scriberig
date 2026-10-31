//! Verifies semantic and opaque-content preservation for staged XLSX edits.

use scriberig_core::EntityId;
use scriberig_domain_spreadsheet::{Cell, CellValue, DEFAULT_CELL_PAGE_LIMIT};
use std::collections::BTreeMap;

use crate::{MAX_XLSX_CELL_EDITS, XlsxAdapterError, XlsxCellEdit, import_xlsx, validate_package};

#[path = "validation/opaque.rs"]
mod opaque;
#[path = "validation/rename.rs"]
mod rename;

pub(super) use opaque::{member_digests, validate_opaque_members, validation_error};

/// Validates one candidate package against its exact base package.
///
/// The validator permits at most the bounded batch size of authored cell changes, requires
/// workbook/sheet/name/table structure to remain stable, and verifies every
/// non-cell package part remains byte-identical after decompression. Formula
/// cache differences do not count as semantic cell changes.
///
/// # Parameters
/// - `old`: Exact base XLSX package bytes.
/// - `new`: Candidate XLSX package bytes.
///
/// # Returns
/// Unit when the package reopens, the semantic edit is local, and opaque content survives.
///
/// # Errors
/// Returns a structured error for invalid packages, broad semantic changes, or lost opaque parts.
pub fn validate_edit(old: &[u8], new: &[u8]) -> Result<(), XlsxAdapterError> {
    validate_package(new)?;
    let workbook_id = EntityId::from_bytes([0; 16]);
    let original = import_xlsx(old, workbook_id)?;
    let candidate = import_xlsx(new, workbook_id)?;
    if !original.editable || !candidate.editable {
        return Err(validation_error(
            "base or candidate package is read-only for loss-preserving edits",
        ));
    }
    if rename::validate_candidate(old, new, &original, &candidate)?.is_some() {
        return Ok(());
    }
    validate_workbook_structure(&original, &candidate)?;
    validate_cell_changes(&original, &candidate)?;
    validate_opaque_members(old, new)
}

/// Validates that a candidate contains the requested single-cell edit.
///
/// # Parameters
/// - `old`: Exact base XLSX package bytes.
/// - `new`: Candidate XLSX package bytes.
/// - `worksheet`: Exact authored worksheet name.
/// - `row`: One-based target row.
/// - `column`: One-based target column.
/// - `expected`: Requested semantic value at the target coordinate.
///
/// # Returns
/// Unit when the requested value is present and no other cell changed.
///
/// # Errors
/// Returns an error when the package or target edit does not match the request.
pub fn validate_cell_edit(
    old: &[u8],
    new: &[u8],
    worksheet: &str,
    row: u32,
    column: u32,
    expected: &CellValue,
) -> Result<(), XlsxAdapterError> {
    validate_edit(old, new)?;
    if row == 0 || column == 0 {
        return Err(validation_error(
            "target cell coordinates must be one-based",
        ));
    }
    let workbook_id = EntityId::from_bytes([0; 16]);
    let candidate = import_xlsx(new, workbook_id)?;
    let target_sheet = candidate
        .worksheets
        .iter()
        .find(|sheet| sheet.name == worksheet)
        .ok_or_else(|| validation_error("candidate is missing the target worksheet"))?;
    let actual = collect_cells(&target_sheet.cells)?
        .remove(&(row, column))
        .map(|cell| cell.value);
    let matches = match (actual.as_ref(), expected) {
        (None, CellValue::Blank) => true,
        (Some(actual), expected) => values_match(actual, expected),
        _ => false,
    };
    if !matches {
        return Err(validation_error(
            "candidate target cell does not match the requested semantic value",
        ));
    }
    Ok(())
}

/// Validates one bounded set of requested cell changes and package preservation.
///
/// # Parameters
/// - `old`: Exact base XLSX package bytes.
/// - `new`: Candidate package bytes.
/// - `worksheet`: Authored worksheet name receiving every edit.
/// - `edits`: Exact expected coordinates and typed values.
///
/// # Returns
/// Unit when every requested value matches and no unrelated cell or package part changes.
///
/// # Errors
/// Returns an error when edits are invalid, targets mismatch, unrelated cells change,
/// workbook structure changes, or opaque package content is not preserved.
pub fn validate_cell_edits(
    old: &[u8],
    new: &[u8],
    worksheet: &str,
    edits: &[XlsxCellEdit],
) -> Result<(), XlsxAdapterError> {
    if edits.is_empty() || edits.len() > MAX_XLSX_CELL_EDITS {
        return Err(validation_error(
            "cell edit count is outside the supported bound",
        ));
    }
    let mut expected = BTreeMap::new();
    for edit in edits {
        if edit.row == 0 || edit.column == 0 || edit.column > 16_384 {
            return Err(validation_error(
                "target cell coordinates are outside the XLSX grid",
            ));
        }
        if expected.insert((edit.row, edit.column), edit).is_some() {
            return Err(validation_error(
                "cell edit set contains duplicate coordinates",
            ));
        }
    }
    validate_package(new)?;
    let workbook_id = EntityId::from_bytes([0; 16]);
    let original = import_xlsx(old, workbook_id)?;
    let candidate = import_xlsx(new, workbook_id)?;
    if !original.editable || !candidate.editable {
        return Err(validation_error(
            "base or candidate package is read-only for safe edits",
        ));
    }
    validate_workbook_structure(&original, &candidate)?;
    let original_target = original
        .worksheets
        .iter()
        .find(|sheet| sheet.name == worksheet)
        .ok_or_else(|| validation_error("exact base worksheet is missing"))?;
    let candidate_target = candidate
        .worksheets
        .iter()
        .find(|sheet| sheet.name == worksheet)
        .ok_or_else(|| validation_error("candidate worksheet is missing"))?;
    for (old_sheet, new_sheet) in original.worksheets.iter().zip(&candidate.worksheets) {
        let old_cells = collect_cells(&old_sheet.cells)?;
        let new_cells = collect_cells(&new_sheet.cells)?;
        let coordinates = old_cells
            .keys()
            .chain(new_cells.keys())
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        for coordinate in coordinates {
            if !cells_match(old_cells.get(&coordinate), new_cells.get(&coordinate))
                && (old_sheet.id != original_target.id || !expected.contains_key(&coordinate))
            {
                return Err(validation_error(
                    "candidate changed an unrequested semantic cell",
                ));
            }
        }
    }
    let cells = collect_cells(&candidate_target.cells)?;
    for (coordinate, edit) in expected {
        let actual = cells.get(&coordinate).map(|cell| &cell.value);
        let matches = match (actual, &edit.value) {
            (None, CellValue::Blank) => true,
            (Some(actual), expected) => values_match(actual, expected),
            _ => false,
        };
        if !matches {
            return Err(validation_error(
                "candidate cell value differs from the requested edit",
            ));
        }
    }
    validate_opaque_members(old, new)
}

/// Checks stable workbook and worksheet collection structure.
///
/// # Parameters
/// - `old`: Imported exact base workbook.
/// - `new`: Imported candidate workbook.
///
/// # Returns
/// Unit when worksheet order, names, defined names, and table metadata match.
///
/// # Errors
/// Returns an error when the candidate changes workbook structure.
fn validate_workbook_structure(
    old: &crate::XlsxImport,
    new: &crate::XlsxImport,
) -> Result<(), XlsxAdapterError> {
    if old.workbook.worksheets != new.workbook.worksheets
        || old.workbook.defined_names != new.workbook.defined_names
        || old.worksheets.len() != new.worksheets.len()
    {
        return Err(validation_error(
            "candidate changed worksheet order, count, or defined names",
        ));
    }
    for (old_sheet, new_sheet) in old.worksheets.iter().zip(&new.worksheets) {
        if old_sheet.id != new_sheet.id
            || old_sheet.name != new_sheet.name
            || old_sheet.tables != new_sheet.tables
        {
            return Err(validation_error(
                "candidate changed worksheet identity, name, or table structure",
            ));
        }
    }
    Ok(())
}

/// Checks that no more than the supported batch size of cell values changes.
///
/// # Parameters
/// - `old`: Imported exact base workbook.
/// - `new`: Imported candidate workbook.
///
/// # Returns
/// Unit when no more than the bounded edit set changes semantic cells.
///
/// # Errors
/// Returns an error when too many cells or any cell styles change.
fn validate_cell_changes(
    old: &crate::XlsxImport,
    new: &crate::XlsxImport,
) -> Result<(), XlsxAdapterError> {
    let mut changes = 0;
    for (old_sheet, new_sheet) in old.worksheets.iter().zip(&new.worksheets) {
        let old_cells = collect_cells(&old_sheet.cells)?;
        let new_cells = collect_cells(&new_sheet.cells)?;
        let coordinates = old_cells
            .keys()
            .chain(new_cells.keys())
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        for coordinate in coordinates {
            let old_cell = old_cells.get(&coordinate);
            let new_cell = new_cells.get(&coordinate);
            if !cells_match(old_cell, new_cell) {
                changes += 1;
                if changes > MAX_XLSX_CELL_EDITS {
                    return Err(validation_error(
                        "candidate changed more cells than one bounded edit set supports",
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Collects sparse worksheet cells through bounded domain pages.
///
/// # Parameters
/// - `cells`: Sparse semantic cell collection.
///
/// # Returns
/// Cells keyed by one-based row and column coordinates.
///
/// # Errors
/// Returns an error when bounded collection paging fails.
fn collect_cells(
    cells: &scriberig_domain_spreadsheet::CellCollection,
) -> Result<BTreeMap<(u32, u32), Cell>, XlsxAdapterError> {
    let mut indexed = BTreeMap::new();
    let mut offset = 0;
    loop {
        let (page, has_more) =
            cells
                .page(None, offset, DEFAULT_CELL_PAGE_LIMIT)
                .map_err(|error| {
                    validation_error(&format!("could not page worksheet cells: {error:?}"))
                })?;
        let count = page.len();
        for cell in page {
            indexed.insert((cell.row, cell.column), cell.clone());
        }
        offset += count;
        if !has_more {
            return Ok(indexed);
        }
    }
}

/// Compares authored cell content while ignoring advisory formula cache values.
///
/// # Parameters
/// - `old`: Base cell, absent for a newly authored cell.
/// - `new`: Candidate cell, absent for a removed cell.
///
/// # Returns
/// True when authored semantic content matches.
fn cells_match(old: Option<&Cell>, new: Option<&Cell>) -> bool {
    match (old, new) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.id == right.id
                && left.worksheet == right.worksheet
                && left.row == right.row
                && left.column == right.column
                && left.style == right.style
                && values_match(&left.value, &right.value)
        }
        _ => false,
    }
}

/// Compares cell values while treating formula results as non-authoritative.
///
/// # Parameters
/// - `old`: Base authored value.
/// - `new`: Candidate authored value.
///
/// # Returns
/// True when scalar values or authored formula expressions match.
fn values_match(old: &CellValue, new: &CellValue) -> bool {
    match (old, new) {
        (CellValue::Formula(old), CellValue::Formula(new)) => old.expression == new.expression,
        _ => old == new,
    }
}
