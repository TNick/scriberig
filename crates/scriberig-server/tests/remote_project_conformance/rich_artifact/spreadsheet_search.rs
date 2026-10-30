//! Asserts embedded and protocol spreadsheet summaries/search agree.

use scriberig_app::ApplicationContext;
use scriberig_core::EntityId;
use scriberig_core::{ProjectId, SnapshotId};
use scriberig_domain_spreadsheet::CellId;
use scriberig_project::{SpreadsheetCellSearch, SpreadsheetWorkbookSummary};
use scriberig_protocol::{SpreadsheetCellFindResponse, SpreadsheetWorkbookResponse};

use super::{EmbeddedApplication, RemoteApplicationClient};

/// Reads workbook metadata and matching cells through embedded and remote APIs.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `workbook`: Stable semantic workbook identity.
///
/// # Returns
/// Stable semantic identity of the workbook's Budget worksheet.
pub(super) fn assert_workbook_and_find(
    embedded: &EmbeddedApplication,
    remote: &RemoteApplicationClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    workbook: EntityId,
) -> EntityId {
    let local = embedded
        .inspect_spreadsheet_workbook(context, project, workbook)
        .expect("embedded workbook summary");
    let worksheet = budget_worksheet(&local);
    let remote_summary = remote
        .inspect_spreadsheet_workbook(
            project.to_string(),
            snapshot.to_string(),
            workbook.to_string(),
        )
        .expect("remote workbook summary");
    assert_workbook_summary(
        &local,
        &remote_summary,
        &project.to_string(),
        &snapshot.to_string(),
        &worksheet.to_string(),
    );
    let request = scriberig_app::SpreadsheetCellSearchRequest {
        project,
        snapshot,
        workbook,
        query: "existing".into(),
        offset: 0,
        limit: 10,
    };
    let local_search = embedded
        .find_spreadsheet_cells(context, &request)
        .expect("embedded cell search");
    let remote_search = remote
        .find_spreadsheet_cells(
            project.to_string(),
            snapshot.to_string(),
            workbook.to_string(),
            "existing",
            0,
            10,
        )
        .expect("remote cell search");
    assert_cell_search(
        &local_search,
        &remote_search,
        &project.to_string(),
        &snapshot.to_string(),
        &workbook.to_string(),
        &worksheet.to_string(),
        &CellId::new(workbook, worksheet, 3, 3).entity().to_string(),
    );
    worksheet
}

/// Compares one bounded spreadsheet range through embedded and remote APIs.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `worksheet`: Stable semantic worksheet identity.
pub(super) fn assert_cell_page(
    embedded: &EmbeddedApplication,
    remote: &RemoteApplicationClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    worksheet: EntityId,
) {
    let range = scriberig_domain_spreadsheet::CellRange {
        first_row: 3,
        first_column: 1,
        last_row: 3,
        last_column: 3,
    };
    let local = embedded
        .read_spreadsheet_cells(context, project, worksheet, Some(&range), 0, 10)
        .expect("embedded sparse cell range");
    let remote = remote
        .read_spreadsheet_cells(
            project.to_string(),
            snapshot.to_string(),
            worksheet.to_string(),
            Some(scriberig_protocol::SpreadsheetRange {
                first_row: 3,
                first_column: 1,
                last_row: 3,
                last_column: 3,
            }),
            0,
            10,
        )
        .expect("remote sparse cell range");
    assert_eq!(remote.total, local.page.total);
    assert_eq!(remote.cells.len(), local.page.cells.len());
    assert_eq!(
        remote
            .cells
            .iter()
            .map(|cell| (cell.id.clone(), cell.row, cell.column))
            .collect::<Vec<_>>(),
        local
            .page
            .cells
            .iter()
            .map(|cell| (cell.id.entity().to_string(), cell.row, cell.column))
            .collect::<Vec<_>>()
    );
}

/// Finds the Budget worksheet identity in an exact workbook summary.
///
/// - `summary`: Embedded workbook summary containing the ordered worksheet list.
///
/// # Returns
/// Stable semantic identity of the authored Budget worksheet.
pub(super) fn budget_worksheet(summary: &SpreadsheetWorkbookSummary) -> EntityId {
    summary
        .worksheets
        .iter()
        .find(|worksheet| worksheet.name == "Budget")
        .expect("Budget worksheet")
        .id
}

/// Compares embedded and remote exact-snapshot workbook summaries.
///
/// - `local`: Embedded semantic workbook summary.
/// - `remote`: Public protocol workbook summary.
/// - `project`: Expected serialized project identity.
/// - `snapshot`: Expected serialized snapshot identity.
/// - `worksheet`: Expected Budget worksheet identity.
pub(super) fn assert_workbook_summary(
    local: &SpreadsheetWorkbookSummary,
    remote: &SpreadsheetWorkbookResponse,
    project: &str,
    snapshot: &str,
    worksheet: &str,
) {
    assert_eq!(remote.project, project);
    assert_eq!(remote.snapshot, snapshot);
    assert_eq!(remote.workbook, local.id.to_string());
    assert_eq!(remote.worksheets.len(), local.worksheets.len());
    assert_eq!(remote.worksheets[0].id, worksheet);
    assert_eq!(remote.worksheets[0].name, local.worksheets[0].name);
    assert_eq!(
        remote.worksheets[0].used_range.as_ref().map(|range| (
            range.first_row,
            range.first_column,
            range.last_row,
            range.last_column,
        )),
        local.worksheets[0].used_range.as_ref().map(|range| (
            range.first_row,
            range.first_column,
            range.last_row,
            range.last_column,
        ))
    );
}

/// Compares embedded and remote semantic handles for one authored cell search.
///
/// - `local`: Embedded exact-snapshot search results.
/// - `remote`: Public protocol search results.
/// - `project`: Expected serialized project identity.
/// - `snapshot`: Expected serialized snapshot identity.
/// - `workbook`: Expected serialized workbook identity.
/// - `worksheet`: Expected stable worksheet identity.
/// - `cell`: Expected stable cell identity.
pub(super) fn assert_cell_search(
    local: &SpreadsheetCellSearch,
    remote: &SpreadsheetCellFindResponse,
    project: &str,
    snapshot: &str,
    workbook: &str,
    worksheet: &str,
    cell: &str,
) {
    assert_eq!(remote.project, project);
    assert_eq!(remote.snapshot, snapshot);
    assert_eq!(remote.workbook, workbook);
    assert_eq!(remote.total, local.total);
    assert_eq!(remote.matches.len(), local.matches.len());
    assert_eq!(remote.matches.len(), 1);
    assert_eq!(remote.matches[0].worksheet, worksheet);
    assert_eq!(remote.matches[0].cell, cell);
    assert_eq!(remote.matches[0].authored_value, "existing");
}
