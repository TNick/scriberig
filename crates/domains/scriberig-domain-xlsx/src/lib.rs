//! Imports and edits XLSX workbooks while retaining unsupported package content.

mod adapter;
mod error;
mod validation;

pub use adapter::{
    MAX_XLSX_CELL_EDITS, XlsxCellEdit, XlsxImport, edit_cell, edit_cells, import_xlsx,
    rename_sheet, semantic_workbook_id, validate_package,
};
pub use error::XlsxAdapterError;
pub use validation::{validate_cell_edit, validate_cell_edits, validate_edit};

/// Stable scanner and artifact-manifest identity for the XLSX adapter.
pub const ADAPTER_ID: &str = "xlsx";

/// Package implementation version recorded in artifact source evidence.
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");
