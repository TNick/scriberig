//! Provides application-backed provider-neutral agent tools.

mod artifact_inspect;
mod cad_edit;
mod cad_layer_change;
mod document_cell_edit;
mod document_edit;
mod document_insert;
mod geo_edit;
mod inspect;
mod qgis_rename;
mod spreadsheet_edit;
mod spreadsheet_read;
mod spreadsheet_rename;

pub use crate::capability::{INSPECT_CAPABILITY, inspect_available};
pub use artifact_inspect::{ARTIFACT_INSPECT_TOOL, ArtifactInspectTool};
pub use cad_edit::{CAD_LINE_EDIT_TOOL, CadLineEditTool};
pub use cad_layer_change::{CAD_LAYER_CHANGE_TOOL, CadLayerChangeTool};
pub use document_cell_edit::{DOCUMENT_TABLE_CELL_EDIT_TOOL, DocumentTableCellEditTool};
pub use document_edit::{DOCUMENT_PARAGRAPH_EDIT_TOOL, DocumentParagraphEditTool};
pub use document_insert::{DOCUMENT_PARAGRAPH_INSERT_TOOL, DocumentParagraphInsertTool};
pub use geo_edit::{
    GEO_ATTRIBUTE_EDIT_TOOL, GEO_GEOMETRY_EDIT_TOOL, GeoAttributeEditTool, GeoGeometryEditTool,
};
pub use inspect::InspectTool;
pub use qgis_rename::{QGIS_LAYER_RENAME_TOOL, QgisLayerRenameTool};
pub use spreadsheet_edit::{SPREADSHEET_CELL_EDIT_TOOL, SpreadsheetCellEditTool};
pub use spreadsheet_read::{SPREADSHEET_RANGE_READ_TOOL, SpreadsheetRangeReadTool};
pub use spreadsheet_rename::{SPREADSHEET_SHEET_RENAME_TOOL, SpreadsheetSheetRenameTool};
