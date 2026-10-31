//! Maps retained-package XLSX data to shared spreadsheet semantics.

use scriberig_core::EntityId;
use scriberig_domain_spreadsheet::{
    Cell, CellCollection, CellId, CellValue, DefinedName, Formula, Range, Table, Workbook,
    Worksheet,
};

use crate::XlsxAdapterError;

#[path = "adapter_write.rs"]
mod write;
pub use write::{
    MAX_XLSX_CELL_EDITS, XlsxCellEdit, edit_cell, edit_cells, rename_sheet, validate_package,
};

/// Reports parsed workbook semantics and package-preserving edit capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XlsxImport {
    /// Generic sparse workbook model.
    pub workbook: Workbook,
    /// Per-worksheet semantic records in source order.
    pub worksheets: Vec<Worksheet>,
    /// Whether the retained OOXML package supports safe cell edits.
    pub editable: bool,
    /// Safe explanation when the package is read-only.
    pub diagnostic: Option<String>,
}

/// Derives the semantic Workbook entity identity from its source artifact.
///
/// # Parameters
/// - `source`: Stable source artifact identity.
///
/// # Returns
/// A deterministic Workbook entity identity.
#[must_use]
pub fn semantic_workbook_id(source: EntityId) -> EntityId {
    derive_id(source, b"spreadsheet.workbook", 0)
}

/// Imports a workbook using exact source bytes and a caller-owned semantic ID.
///
/// # Parameters
/// - `bytes`: Exact XLSX package bytes.
/// - `workbook_id`: Stable semantic workbook identity.
///
/// # Returns
/// Sparse generic workbook semantics and adapter capability.
///
/// # Errors
/// Returns a structured parse or conversion diagnostic.
pub fn import_xlsx(bytes: &[u8], workbook_id: EntityId) -> Result<XlsxImport, XlsxAdapterError> {
    let spreadsheet = rxls::Spreadsheet::open(bytes).map_err(|error| XlsxAdapterError {
        code: "xlsx_parse_failed",
        message: error.to_string(),
    })?;
    let source = spreadsheet.workbook();
    let worksheets = source
        .sheets
        .iter()
        .enumerate()
        .filter(|(_, sheet)| sheet.is_worksheet)
        .map(|(index, sheet)| import_worksheet(sheet, index, workbook_id))
        .collect::<Result<Vec<_>, XlsxAdapterError>>()?;
    let defined_names = import_defined_names(source, workbook_id)?;
    let workbook = Workbook {
        id: workbook_id,
        worksheets: worksheets.iter().map(|sheet| sheet.id).collect(),
        defined_names,
    };
    let (editable, diagnostic) = match spreadsheet.edit_capability() {
        rxls::EditCapability::ReadWrite => (true, None),
        rxls::EditCapability::ReadOnly(reason) => (
            false,
            Some(format!("XLSX package is read-only: {reason:?}")),
        ),
    };
    Ok(XlsxImport {
        workbook,
        worksheets,
        editable,
        diagnostic,
    })
}

/// Imports one worksheet's sparse cells, extent, and named tables.
///
/// # Parameters
/// - `sheet`: Parser-owned source worksheet.
/// - `index`: Stable source-order worksheet position.
/// - `workbook_id`: Owning semantic workbook identity.
///
/// # Returns
/// Shared semantic worksheet.
///
/// # Errors
/// Returns an error when cell coordinates are invalid for the sparse domain.
fn import_worksheet(
    sheet: &rxls::Sheet,
    index: usize,
    workbook_id: EntityId,
) -> Result<Worksheet, XlsxAdapterError> {
    let worksheet_id = derive_id(workbook_id, b"spreadsheet.worksheet", index as u64);
    let cells = sheet
        .cells()
        .map(|(row, column, value)| {
            let row = row.saturating_add(1);
            let column = u32::from(column).saturating_add(1);
            Cell {
                id: CellId::new(workbook_id, worksheet_id, row, column),
                worksheet: worksheet_id,
                row,
                column,
                value: map_cell(value),
                style: None,
            }
        })
        .collect::<Vec<_>>();
    let cells = CellCollection::new(workbook_id, worksheet_id, cells).map_err(|error| {
        XlsxAdapterError {
            code: "xlsx_cell_coordinates_invalid",
            message: format!("worksheet {}: {error:?}", sheet.name),
        }
    })?;
    let used_range = sheet
        .dimensions()
        .map(|(first_row, first_col, last_row, last_col)| Range {
            worksheet: worksheet_id,
            first_row: first_row.saturating_add(1),
            first_column: u32::from(first_col).saturating_add(1),
            last_row: last_row.saturating_add(1),
            last_column: u32::from(last_col).saturating_add(1),
        });
    let tables = sheet
        .tables()
        .iter()
        .map(|table| {
            let (first_row, first_column, last_row, last_column) = table.range;
            Table {
                id: derive_named_id(worksheet_id, b"spreadsheet.table", table.name.as_bytes()),
                name: table.name.clone(),
                range: Range {
                    worksheet: worksheet_id,
                    first_row: first_row.saturating_add(1),
                    first_column: u32::from(first_column).saturating_add(1),
                    last_row: last_row.saturating_add(1),
                    last_column: u32::from(last_column).saturating_add(1),
                },
            }
        })
        .collect();
    Ok(Worksheet {
        id: worksheet_id,
        workbook: workbook_id,
        name: sheet.name.clone(),
        cells,
        used_range,
        tables,
    })
}

/// Imports workbook names and resolves each sheet-scoped name to a semantic worksheet.
///
/// # Parameters
/// - `source`: Parsed workbook metadata.
/// - `workbook_id`: Stable semantic workbook identity.
///
/// # Returns
/// Workbook-scoped and worksheet-scoped defined names.
///
/// # Errors
/// Returns an error when a local name refers to a missing worksheet.
fn import_defined_names(
    source: &rxls::Workbook,
    workbook_id: EntityId,
) -> Result<Vec<DefinedName>, XlsxAdapterError> {
    let mut names = source
        .defined_names
        .iter()
        .map(|(name, expression)| DefinedName {
            name: name.clone(),
            expression: expression.clone(),
            scope: None,
        })
        .collect::<Vec<_>>();
    for name in &source.local_defined_names {
        let sheet_index = source
            .sheets
            .iter()
            .position(|sheet| sheet.is_worksheet && sheet.name == name.sheet)
            .ok_or_else(|| XlsxAdapterError {
                code: "xlsx_defined_name_scope_missing",
                message: format!(
                    "defined name {} refers to missing sheet {}",
                    name.name, name.sheet
                ),
            })?;
        names.push(DefinedName {
            name: name.name.clone(),
            expression: name.refers_to.clone(),
            scope: Some(derive_id(
                workbook_id,
                b"spreadsheet.worksheet",
                sheet_index as u64,
            )),
        });
    }
    Ok(names)
}

/// Converts one parser-native cell into the shared spreadsheet value model.
///
/// # Parameters
/// - `value`: Parser-native cell value.
///
/// # Returns
/// Adapter-neutral typed cell value.
fn map_cell(value: &rxls::Cell) -> CellValue {
    match value {
        rxls::Cell::Text(value) => CellValue::Text(value.clone()),
        rxls::Cell::Number(value) | rxls::Cell::Date(value) => CellValue::Number(value.to_string()),
        rxls::Cell::Bool(value) => CellValue::Boolean(*value),
        rxls::Cell::Error(value) => CellValue::Error(value.clone()),
        rxls::Cell::Formula { formula, cached } => CellValue::Formula(Formula {
            expression: format!("={formula}"),
            cached_value: Some(Box::new(map_cell(cached))),
        }),
    }
}

/// Derives an adapter-owned semantic identity from a parent, namespace, and ordinal.
///
/// # Parameters
/// - `parent`: Parent semantic identity.
/// - `namespace`: Stable adapter namespace.
/// - `ordinal`: Stable source-order position.
///
/// # Returns
/// A deterministic semantic identity.
fn derive_id(parent: EntityId, namespace: &[u8], ordinal: u64) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(parent.as_bytes());
    hasher.update(namespace);
    hasher.update(&ordinal.to_be_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Derives a stable adapter identity from a parent, namespace, and authored key.
///
/// # Parameters
/// - `parent`: Owning semantic identity.
/// - `namespace`: Stable adapter namespace.
/// - `key`: Authored key unique within the parent and namespace.
///
/// # Returns
/// A deterministic semantic identity unaffected by sibling ordering.
#[must_use]
fn derive_named_id(parent: EntityId, namespace: &[u8], key: &[u8]) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(parent.as_bytes());
    hasher.update(namespace);
    hasher.update(key);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

/// Creates a stable invalid-cell diagnostic.
///
/// # Parameters
/// - `message`: Safe validation detail.
///
/// # Returns
/// A structured invalid-cell failure.
pub(super) fn invalid_cell(message: &str) -> XlsxAdapterError {
    XlsxAdapterError {
        code: "xlsx_cell_invalid",
        message: message.into(),
    }
}

/// Maps one retained-package parser error to a safe adapter error.
///
/// # Parameters
/// - `error`: Parser-native failure.
///
/// # Returns
/// A structured XLSX operation failure.
pub(super) fn map_error(error: &rxls::Error) -> XlsxAdapterError {
    XlsxAdapterError {
        code: "xlsx_operation_failed",
        message: error.to_string(),
    }
}
