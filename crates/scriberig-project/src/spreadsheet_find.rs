//! Finds authored cell content across an exact-snapshot workbook.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_spreadsheet::{Cell, CellIndex, CellValue};
use scriberig_store::{RevisionStore, SnapshotStore};

use super::{StoredWorksheetSummary, read_error};
use crate::{ProjectError, ProjectService};

/// Maximum number of spreadsheet search matches returned on one page.
pub const MAX_SPREADSHEET_SEARCH_PAGE: usize = 100;

/// Identifies one matching authored spreadsheet cell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellMatch {
    /// Stable semantic worksheet identity.
    pub worksheet: EntityId,
    /// Stable semantic cell identity.
    pub cell: EntityId,
    /// One-based cell row.
    pub row: u32,
    /// One-based cell column.
    pub column: u32,
    /// Authored text, exact number, boolean, error, or formula expression.
    pub authored_value: String,
}

/// Contains a bounded page of matches at one exact workbook snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellSearch {
    /// Project owning the workbook.
    pub project: ProjectId,
    /// Exact immutable snapshot used for every match.
    pub snapshot: SnapshotId,
    /// Stable semantic workbook identity.
    pub workbook: EntityId,
    /// Matching cells in workbook worksheet and row-major order.
    pub matches: Vec<SpreadsheetCellMatch>,
    /// Total matches before pagination.
    pub total: usize,
    /// Whether another page contains matches.
    pub has_more: bool,
}

impl ProjectService {
    /// Finds authored cell values at an exact workbook snapshot.
    ///
    /// Matching is case-insensitive substring search over authored text,
    /// exact number/boolean/error values, and formula expressions. Cached
    /// formula results are excluded because they are not authoritative.
    ///
    /// # Parameters
    /// - `project`: Project owning the workbook.
    /// - `snapshot`: Exact immutable snapshot to inspect.
    /// - `workbook`: Stable semantic workbook identity.
    /// - `query`: Non-empty authored-value substring to find.
    /// - `offset`: Zero-based result offset.
    /// - `limit`: Maximum matches, from one through one hundred.
    ///
    /// # Returns
    /// Bounded stable semantic cell handles and continuation state.
    ///
    /// # Errors
    /// Returns stale-snapshot, malformed workbook/cell, invalid query/page,
    /// or ownership errors.
    pub fn find_spreadsheet_cells(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        workbook: EntityId,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> Result<SpreadsheetCellSearch, ProjectError> {
        if !(1..=MAX_SPREADSHEET_SEARCH_PAGE).contains(&limit) {
            return Err(read_error(
                "invalid_page_size",
                "spreadsheet search limit must be between one and one hundred",
            ));
        }
        if query.trim().is_empty() {
            return Err(read_error(
                "invalid_query",
                "spreadsheet search text is empty",
            ));
        }
        let summary = self.spreadsheet_workbook_summary(project, snapshot, workbook)?;
        let needle = query.to_lowercase();
        let mut matches = Vec::new();
        let mut total = 0;
        for worksheet in &summary.worksheets {
            self.collect_worksheet_matches(
                project,
                snapshot,
                worksheet.id,
                &needle,
                offset,
                limit,
                &mut total,
                &mut matches,
            )?;
        }
        if offset > total {
            return Err(read_error(
                "offset_out_of_range",
                "spreadsheet search offset exceeds the matching cell count",
            ));
        }
        Ok(SpreadsheetCellSearch {
            project,
            snapshot,
            workbook,
            has_more: total > offset.saturating_add(matches.len()),
            matches,
            total,
        })
    }

    /// Adds one worksheet's matches while retaining only the requested result page.
    ///
    /// - `project`: Project owning the workbook.
    /// - `snapshot`: Exact immutable snapshot being searched.
    /// - `worksheet`: Semantic worksheet identity to scan.
    /// - `needle`: Lowercase authored-value substring.
    /// - `offset`: Number of workbook matches to skip.
    /// - `limit`: Maximum result page size.
    /// - `total`: Running match count across prior worksheets.
    /// - `matches`: Bounded destination page.
    ///
    /// # Returns
    /// Unit after every indexed cell page in the worksheet has been searched.
    ///
    /// # Errors
    /// Returns storage, stale-snapshot, or malformed-cell errors from a page read.
    #[allow(clippy::too_many_arguments)]
    fn collect_worksheet_matches(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        worksheet: EntityId,
        needle: &str,
        offset: usize,
        limit: usize,
        total: &mut usize,
        matches: &mut Vec<SpreadsheetCellMatch>,
    ) -> Result<(), ProjectError> {
        let index = self.worksheet_cell_index(project, snapshot, worksheet)?;
        let mut page_offset = 0;
        loop {
            let (cells, _, has_more) = self.read_indexed_spreadsheet_cells(
                snapshot,
                worksheet,
                &index,
                None,
                page_offset,
                crate::MAX_SPREADSHEET_CELL_PAGE,
            )?;
            page_offset += cells.len();
            Self::add_page_matches(worksheet, &cells, needle, offset, limit, total, matches);
            if !has_more {
                return Ok(());
            }
        }
    }

    /// Loads one snapshot-local cell membership index for efficient page scanning.
    ///
    /// - `project`: Project owning the worksheet.
    /// - `snapshot`: Exact snapshot whose membership is required.
    /// - `worksheet`: Semantic worksheet identity.
    ///
    /// # Returns
    /// Validated stored or compatibility cell index for the worksheet.
    ///
    /// # Errors
    /// Returns ownership, stale-snapshot, storage, or malformed-index errors.
    fn worksheet_cell_index(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        worksheet: EntityId,
    ) -> Result<CellIndex, ProjectError> {
        let owned = self
            .store
            .project_entity(project, worksheet)
            .map_err(crate::errors::storage_error)?
            .ok_or_else(|| {
                read_error(
                    "worksheet_not_found",
                    "worksheet is not owned by the project",
                )
            })?;
        if owned.kind != "spreadsheet.worksheet" {
            return Err(read_error(
                "worksheet_not_found",
                "semantic entity is not a worksheet",
            ));
        }
        let revision_id = self
            .store
            .resolve(snapshot, worksheet)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                read_error(
                    "worksheet_not_in_snapshot",
                    "worksheet is absent from the requested snapshot",
                )
            })?;
        let revision = self
            .store
            .get(revision_id)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                read_error(
                    "worksheet_revision_missing",
                    "worksheet revision is unavailable",
                )
            })?;
        let metadata: StoredWorksheetSummary =
            serde_json::from_slice(revision.payload()).map_err(|_| {
                read_error(
                    "worksheet_revision_invalid",
                    "worksheet metadata is invalid",
                )
            })?;
        if metadata.id != worksheet {
            return Err(read_error(
                "worksheet_owner_mismatch",
                "worksheet metadata does not match its semantic identity",
            ));
        }
        match metadata.cell_index {
            Some(index) => Ok(index),
            None => {
                self.read_legacy_spreadsheet_cell_index(project, snapshot, &owned.relative_path)
            }
        }
    }

    /// Appends matching cells from one bounded semantic page.
    ///
    /// - `worksheet`: Semantic worksheet identity owning these cells.
    /// - `cells`: Bounded exact-snapshot cell values.
    /// - `needle`: Lowercase authored-value substring.
    /// - `offset`: Number of workbook matches to skip.
    /// - `limit`: Maximum result page size.
    /// - `total`: Running match count.
    /// - `matches`: Bounded destination page.
    fn add_page_matches(
        worksheet: EntityId,
        cells: &[Cell],
        needle: &str,
        offset: usize,
        limit: usize,
        total: &mut usize,
        matches: &mut Vec<SpreadsheetCellMatch>,
    ) {
        for cell in cells {
            let Some(authored_value) = searchable_value(cell) else {
                continue;
            };
            if !authored_value.to_lowercase().contains(needle) {
                continue;
            }
            if *total >= offset && matches.len() < limit {
                matches.push(SpreadsheetCellMatch {
                    worksheet,
                    cell: cell.id.entity(),
                    row: cell.row,
                    column: cell.column,
                    authored_value,
                });
            }
            *total += 1;
        }
    }
}

/// Returns authored cell text while excluding non-authoritative cached values.
///
/// - `cell`: Semantic cell whose authored value is inspected.
///
/// # Returns
/// Searchable authored content, or `None` for an explicit blank cell.
fn searchable_value(cell: &Cell) -> Option<String> {
    match &cell.value {
        CellValue::Blank => None,
        CellValue::Text(value) | CellValue::Number(value) | CellValue::Error(value) => {
            Some(value.clone())
        }
        CellValue::Boolean(value) => Some(value.to_string()),
        CellValue::Formula(formula) => Some(formula.expression.clone()),
    }
}
