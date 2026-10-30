//! Defines bounded projection anchors carried beside semantic identity.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
/// Describes a typed view location without replacing semantic entity identity.
///
/// Public members: text, page, spreadsheet, geometry, and opaque anchor variants.
/// Private members: none.
pub enum ProjectionAnchor {
    /// Character or byte span in a text projection.
    TextRange {
        /// Inclusive range start.
        start: u64,
        /// Exclusive range end.
        end: u64,
    },
    /// Region in a paginated document view.
    PageRegion {
        /// One-based page number.
        page: u32,
        /// Horizontal position.
        x: u32,
        /// Vertical position.
        y: u32,
        /// Region width.
        width: u32,
        /// Region height.
        height: u32,
    },
    /// One cell in an exact semantic worksheet.
    SpreadsheetCell {
        /// Stable worksheet entity identity.
        worksheet_id: String,
        /// One-based row.
        row: u32,
        /// One-based column.
        column: u32,
    },
    /// Inclusive cell range in an exact semantic worksheet.
    SpreadsheetRange {
        /// Stable worksheet entity identity.
        worksheet_id: String,
        /// First one-based row.
        first_row: u32,
        /// First one-based column.
        first_column: u32,
        /// Last one-based row.
        last_row: u32,
        /// Last one-based column.
        last_column: u32,
    },
    /// CRS-aware geometry bounds using exact decimal text.
    GeometryBounds {
        /// Coordinate reference system identifier, at most 256 bytes.
        crs: String,
        /// Minimum x coordinate as exact decimal text.
        min_x: String,
        /// Minimum y coordinate as exact decimal text.
        min_y: String,
        /// Maximum x coordinate as exact decimal text.
        max_x: String,
        /// Maximum y coordinate as exact decimal text.
        max_y: String,
    },
    /// Projection-space bounds for one selected CAD entity.
    CadBounds {
        /// Stable semantic drawing identity.
        drawing_id: String,
        /// Model, paper, or named CAD space identity.
        space: String,
        /// Minimum x coordinate as exact drawing-unit text.
        min_x: String,
        /// Minimum y coordinate as exact drawing-unit text.
        min_y: String,
        /// Maximum x coordinate as exact drawing-unit text.
        max_x: String,
        /// Maximum y coordinate as exact drawing-unit text.
        max_y: String,
    },
    /// Adapter-defined anchor with bounded opaque data.
    Opaque {
        /// Adapter identity, at most 128 bytes.
        adapter: String,
        /// Adapter-owned anchor kind, at most 128 bytes.
        anchor_kind: String,
        /// Adapter-owned bounded value, at most 4096 bytes.
        value: String,
    },
}

impl ProjectionAnchor {
    /// Checks coordinate ordering and bounds variable-length descriptor fields.
    ///
    /// # Returns
    /// Unit when this anchor is valid and bounded.
    ///
    /// # Errors
    /// Returns a short diagnostic when coordinates are invalid or strings exceed limits.
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::TextRange { start, end } if end < start => Err("text range is reversed"),
            Self::PageRegion {
                page,
                width,
                height,
                ..
            } if *page == 0 || *width == 0 || *height == 0 => {
                Err("page region coordinates must be positive")
            }
            Self::SpreadsheetCell {
                worksheet_id,
                row,
                column,
            } if worksheet_id.is_empty()
                || worksheet_id.len() > 128
                || *row == 0
                || *column == 0 =>
            {
                Err("spreadsheet cell anchor values are invalid")
            }
            Self::SpreadsheetRange {
                worksheet_id,
                first_row,
                first_column,
                last_row,
                last_column,
                ..
            } if worksheet_id.is_empty()
                || worksheet_id.len() > 128
                || *first_row == 0
                || *first_column == 0
                || first_row > last_row
                || first_column > last_column =>
            {
                Err("spreadsheet range coordinates are invalid")
            }
            Self::GeometryBounds {
                crs,
                min_x,
                min_y,
                max_x,
                max_y,
            } if crs.is_empty()
                || crs.len() > 256
                || [min_x, min_y, max_x, max_y]
                    .iter()
                    .any(|coordinate| coordinate.is_empty() || coordinate.len() > 128) =>
            {
                Err("geometry anchor values exceed descriptor bounds")
            }
            Self::CadBounds {
                drawing_id,
                space,
                min_x,
                min_y,
                max_x,
                max_y,
            } if drawing_id.is_empty()
                || drawing_id.len() > 128
                || space.is_empty()
                || space.len() > 128
                || [min_x, min_y, max_x, max_y]
                    .iter()
                    .any(|coordinate| coordinate.is_empty() || coordinate.len() > 128) =>
            {
                Err("CAD anchor values exceed descriptor bounds")
            }
            Self::Opaque {
                adapter,
                anchor_kind,
                value,
            } if adapter.is_empty()
                || anchor_kind.is_empty()
                || adapter.len() > 128
                || anchor_kind.len() > 128
                || value.len() > 4096 =>
            {
                Err("opaque anchor values exceed descriptor bounds")
            }
            _ => Ok(()),
        }
    }
}
