//! Reads snapshot-bound sparse spreadsheet cell pages from semantic revisions.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_spreadsheet::{Cell, CellIndex, CellIndexEntry, CellRange, Range, Table};
use scriberig_store::{RevisionStore, SnapshotStore};

use crate::ArtifactCapability;
use crate::{ProjectError, ProjectService};

#[path = "spreadsheet_find.rs"]
mod find;
#[path = "spreadsheet_workbook_read.rs"]
mod workbook_read;
pub use find::{MAX_SPREADSHEET_SEARCH_PAGE, SpreadsheetCellMatch, SpreadsheetCellSearch};

/// Maximum cells returned by one spreadsheet page.
pub const MAX_SPREADSHEET_CELL_PAGE: usize = 500;

/// Describes one worksheet in an exact-snapshot workbook summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetWorksheetSummary {
    /// Stable semantic worksheet identity.
    pub id: EntityId,
    /// Authored worksheet name.
    pub name: String,
    /// Bounded authored cell extent when available.
    pub used_range: Option<Range>,
    /// Named semantic tables and their snapshot-bound worksheet ranges.
    pub tables: Vec<Table>,
}

/// Describes a workbook and its ordered worksheets at one exact snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetWorkbookSummary {
    /// Stable semantic workbook identity.
    pub id: EntityId,
    /// Edit capability of the exact source artifact, when catalogued.
    pub artifact_capability: Option<ArtifactCapability>,
    /// Exact semantic snapshot used to resolve worksheet revisions.
    pub snapshot: SnapshotId,
    /// Worksheets in workbook collection order.
    pub worksheets: Vec<SpreadsheetWorksheetSummary>,
}

/// Describes one bounded page of snapshot-bound worksheet cells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpreadsheetCellPage {
    /// Exact source snapshot used for all cell revisions.
    pub snapshot: SnapshotId,
    /// Stable semantic worksheet identity.
    pub worksheet: EntityId,
    /// Edit capability of the exact source artifact, when catalogued.
    pub artifact_capability: Option<ArtifactCapability>,
    /// Adapter diagnostic explaining restricted edit capability, when present.
    pub artifact_diagnostic: Option<String>,
    /// Sparse cells ordered by row then column.
    pub cells: Vec<Cell>,
    /// Total sparse cells in the worksheet at this snapshot.
    pub total: usize,
    /// Whether cells remain after this page.
    pub has_more: bool,
}

impl ProjectService {
    /// Reads one bounded page of sparse worksheet cells from an exact snapshot.
    ///
    /// # Parameters
    /// - `project`: Project owning the worksheet.
    /// - `snapshot`: Exact immutable snapshot to inspect.
    /// - `worksheet`: Stable semantic worksheet identity.
    /// - `range`: Optional inclusive rectangular bounds within the worksheet.
    /// - `offset`: Zero-based offset in row/column order.
    /// - `limit`: Page size from one through five hundred.
    ///
    /// # Returns
    /// Sparse semantic cell page with exact total and continuation state.
    ///
    /// # Errors
    /// Returns a project, storage, decoding, or request validation error.
    pub fn spreadsheet_cell_page(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        worksheet: EntityId,
        range: Option<&CellRange>,
        offset: usize,
        limit: usize,
    ) -> Result<SpreadsheetCellPage, ProjectError> {
        if !(1..=MAX_SPREADSHEET_CELL_PAGE).contains(&limit) {
            return Err(read_error(
                "invalid_page_size",
                "spreadsheet page limit must be between one and five hundred",
            ));
        }
        if range.is_some_and(|range| {
            range.first_row == 0
                || range.first_column == 0
                || range.first_row > range.last_row
                || range.first_column > range.last_column
        }) {
            return Err(read_error(
                "invalid_range",
                "spreadsheet range is invalid for this worksheet",
            ));
        }
        let sheet = self
            .store
            .project_entity(project, worksheet)
            .map_err(crate::errors::storage_error)?
            .ok_or_else(|| {
                read_error(
                    "worksheet_not_found",
                    "worksheet is not owned by the project",
                )
            })?;
        if sheet.kind != "spreadsheet.worksheet" {
            return Err(read_error(
                "worksheet_not_found",
                "semantic entity is not a worksheet",
            ));
        }
        let fidelity = self.artifact_fidelity_for_path(project, &sheet.relative_path)?;
        let artifact_capability = fidelity.as_ref().map(|value| value.capability);
        let artifact_diagnostic = fidelity.and_then(|value| value.diagnostic);
        let sheet_revision = self
            .store
            .resolve(snapshot, worksheet)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                read_error(
                    "worksheet_not_in_snapshot",
                    "worksheet is absent from the requested snapshot",
                )
            })?;
        let sheet_revision = self
            .store
            .get(sheet_revision)
            .map_err(crate::errors::storage_debug_error)?
            .ok_or_else(|| {
                read_error(
                    "worksheet_revision_missing",
                    "worksheet revision is unavailable",
                )
            })?;
        let metadata: StoredWorksheetSummary = serde_json::from_slice(sheet_revision.payload())
            .map_err(|_| {
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
        let index = match metadata.cell_index {
            Some(index) => index,
            None => {
                self.read_legacy_spreadsheet_cell_index(project, snapshot, &sheet.relative_path)?
            }
        };
        let (cells, total, has_more) =
            self.read_indexed_spreadsheet_cells(snapshot, worksheet, &index, range, offset, limit)?;
        Ok(SpreadsheetCellPage {
            snapshot,
            worksheet,
            artifact_capability,
            artifact_diagnostic,
            cells,
            total,
            has_more,
        })
    }

    /// Resolves only the selected snapshot-local cell handles into semantic values.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot for all selected cell revisions.
    /// - `worksheet`: Expected owning worksheet identity.
    /// - `index`: Validated snapshot-specific membership index.
    /// - `range`: Optional inclusive rectangular bounds.
    /// - `offset`: Number of matching cells to skip.
    /// - `limit`: Maximum selected cell count.
    ///
    /// # Returns
    /// Bounded semantic cells, total matches, and continuation state.
    ///
    /// # Errors
    /// Returns an error when the index, offset, snapshot, or cell revision is invalid.
    fn read_indexed_spreadsheet_cells(
        &self,
        snapshot: SnapshotId,
        worksheet: EntityId,
        index: &CellIndex,
        range: Option<&CellRange>,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<Cell>, usize, bool), ProjectError> {
        let (handles, total, has_more) = index
            .page(range, offset, limit)
            .map_err(|_| read_error("invalid_cell_index", "worksheet cell index is invalid"))?;
        if offset > total {
            return Err(read_error(
                "offset_out_of_range",
                "spreadsheet page offset exceeds the cell count",
            ));
        }
        let mut cells = Vec::with_capacity(handles.len());
        for handle in handles {
            let revision_id = self
                .store
                .resolve(snapshot, handle.entity)
                .map_err(crate::errors::storage_debug_error)?
                .ok_or_else(|| {
                    read_error(
                        "cell_not_in_snapshot",
                        "indexed cell is absent from the requested snapshot",
                    )
                })?;
            let revision = self
                .store
                .get(revision_id)
                .map_err(crate::errors::storage_debug_error)?
                .ok_or_else(|| {
                    read_error("cell_revision_missing", "cell revision is unavailable")
                })?;
            let cell: Cell = serde_json::from_slice(revision.payload()).map_err(|_| {
                read_error("cell_revision_invalid", "cell semantic data is invalid")
            })?;
            if cell.worksheet != worksheet || cell.row != handle.row || cell.column != handle.column
            {
                return Err(read_error(
                    "cell_index_mismatch",
                    "indexed cell does not match its worksheet coordinates",
                ));
            }
            cells.push(cell);
        }
        Ok((cells, total, has_more))
    }

    /// Builds a snapshot-specific membership index for pre-index worksheet revisions.
    ///
    /// Older worksheet payloads did not preserve cell membership. Their compatibility path
    /// therefore checks project-owned cell revisions and includes only those present at the
    /// requested snapshot.
    ///
    /// # Parameters
    /// - `project`: Project owning worksheet cells.
    /// - `snapshot`: Exact snapshot used to select existing cell revisions.
    /// - `relative_path`: Source artifact path shared by worksheet cell entities.
    ///
    /// # Returns
    /// Coordinate ordered index of the cells present at the requested snapshot.
    ///
    /// # Errors
    /// Returns an error when ownership, storage, or semantic cell data is invalid.
    fn read_legacy_spreadsheet_cell_index(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        relative_path: &str,
    ) -> Result<CellIndex, ProjectError> {
        let entities = self
            .store
            .project_entities(project)
            .map_err(crate::errors::storage_error)?;
        let mut entries = Vec::new();
        for entity in entities.iter().filter(|entity| {
            entity.kind == "spreadsheet.cell" && entity.relative_path == relative_path
        }) {
            let Some(revision_id) = self
                .store
                .resolve(snapshot, entity.entity)
                .map_err(crate::errors::storage_debug_error)?
            else {
                continue;
            };
            let revision = self
                .store
                .get(revision_id)
                .map_err(crate::errors::storage_debug_error)?
                .ok_or_else(|| {
                    read_error("cell_revision_missing", "cell revision is unavailable")
                })?;
            let cell: Cell = serde_json::from_slice(revision.payload()).map_err(|_| {
                read_error("cell_revision_invalid", "cell semantic data is invalid")
            })?;
            entries.push(CellIndexEntry {
                entity: entity.entity,
                row: cell.row,
                column: cell.column,
            });
        }
        CellIndex::new(entries).map_err(|_| {
            read_error(
                "invalid_cell_index",
                "legacy worksheet cells contain invalid coordinates",
            )
        })
    }
}

/// Stores bounded worksheet metadata in project semantic revisions.
#[derive(serde::Deserialize)]
struct StoredWorksheetSummary {
    /// Stable semantic worksheet identity recorded in the revision.
    id: EntityId,
    /// Owning workbook identity recorded in the revision.
    workbook: EntityId,
    /// Authored worksheet name.
    name: String,
    /// Bounded authored cell extent, when available.
    used_range: Option<Range>,
    /// Named semantic tables stored with this worksheet revision.
    #[serde(default)]
    tables: Vec<Table>,
    /// Snapshot-local cell membership index, absent in older project revisions.
    #[serde(default)]
    cell_index: Option<CellIndex>,
}

/// Creates a structured sparse spreadsheet read failure.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// Project read error without source bytes.
fn read_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
