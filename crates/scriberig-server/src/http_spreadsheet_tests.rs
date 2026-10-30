//! Verifies the local HTTP adapter injects trusted spreadsheet project grants.

use std::sync::Arc;

use super::{HttpState, dispatch};
use crate::development::ensure_fixture;
use scriberig_app::{ApplicationService, StoreInspectionBackend};
use scriberig_core::EntityId;
use scriberig_domain_spreadsheet::CellId;
use scriberig_protocol::{
    Response, ResponseEnvelope, SpreadsheetCandidateValidation, SpreadsheetCellEdit,
    SpreadsheetCellFindRequest, SpreadsheetCellInspectRequest, SpreadsheetCellValue,
    SpreadsheetCellsRequest, SpreadsheetEditRequest, SpreadsheetRange, SpreadsheetRangeEditRequest,
    SpreadsheetRenameRequest, SpreadsheetWorkbookRequest,
};

#[allow(
    clippy::too_many_lines,
    reason = "one end-to-end scenario checks HTTP authorization, paging, workbook summary, cell inspection, candidate, and no-live-write behavior"
)]
/// Reads a real imported XLSX cell through the trusted local HTTP operation.
#[test]
fn http_spreadsheet_read_resolves_local_project_grant_from_host_catalog() {
    let root = std::env::temp_dir().join(format!("scriberig-http-xlsx-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    let mut workbook = rxls::Workbook::new();
    let budget = workbook.add_sheet("Budget");
    budget.write_number(2, 0, 12.0);
    budget.add_table(rxls::Table::new((0, 0, 2, 0), "BudgetTable", ["Amount"]));
    let workbook_path = root.join("budget.xlsx");
    let original = workbook.to_xlsx_checked().expect("serialize workbook");
    std::fs::write(&workbook_path, &original).expect("write workbook");
    let database = root.with_extension("sqlite");
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(&database).expect("database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let imported = projects.import(scan.project.id).expect("import workbook");
    let worksheet = store
        .project_entities(scan.project.id)
        .expect("worksheet ownership")
        .into_iter()
        .find(|entity| entity.kind == "spreadsheet.worksheet")
        .expect("worksheet")
        .entity;
    let workbook = store
        .project_entities(scan.project.id)
        .expect("workbook ownership")
        .into_iter()
        .find(|entity| entity.kind == "spreadsheet.workbook")
        .expect("workbook")
        .entity;
    let service = ApplicationService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects);
    let state = HttpState {
        service,
        principal: scriberig_app::Principal::new("local-user"),
        fixture: ensure_fixture(&store).expect("development fixture"),
        allowed_origin: None,
        project_store: Some(Arc::clone(&store)),
        allowed_project_roots: vec![root.clone()],
        plan_lifecycle_handler: None,
        agent_execution_handler: None,
        provider_readiness_handler: None,
    };
    let request = SpreadsheetCellsRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        worksheet: worksheet.to_string(),
        range: Some(SpreadsheetRange {
            first_row: 3,
            first_column: 1,
            last_row: 3,
            last_column: 1,
        }),
        offset: 0,
        limit: 10,
    };
    let body = serde_json::to_string(&request).expect("encode request");
    let response =
        dispatch(&state, "spreadsheet_cells", &body, "http-xlsx".into()).expect("dispatch request");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let page = match envelope.response {
        Response::SpreadsheetCells(page) => page,
        other => panic!("expected spreadsheet cell page, got {other:?}"),
    };
    assert_eq!(page.snapshot, imported.snapshot.to_string());
    assert_eq!(page.cells.len(), 1);
    assert_eq!(page.cells[0].row, 3);

    let summary_request = SpreadsheetWorkbookRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        workbook: workbook.to_string(),
    };
    let body = serde_json::to_string(&summary_request).expect("encode workbook request");
    let response = dispatch(
        &state,
        "spreadsheet_workbook",
        &body,
        "http-workbook".into(),
    )
    .expect("dispatch workbook summary");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let summary = match envelope.response {
        Response::SpreadsheetWorkbook(summary) => summary,
        other => panic!("expected workbook summary, got {other:?}"),
    };
    assert_eq!(summary.snapshot, imported.snapshot.to_string());
    assert_eq!(summary.worksheets[0].name, "Budget");
    assert_eq!(summary.worksheets[0].tables.len(), 1);
    assert_eq!(summary.worksheets[0].tables[0].name, "BudgetTable");
    assert_eq!(
        summary.worksheets[0].used_range.as_ref().unwrap().last_row,
        3
    );

    let find_request = SpreadsheetCellFindRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        workbook: workbook.to_string(),
        query: "12".into(),
        offset: 0,
        limit: 10,
    };
    let body = serde_json::to_string(&find_request).expect("encode spreadsheet search");
    let response = dispatch(
        &state,
        "spreadsheet_cell_find",
        &body,
        "http-cell-find".into(),
    )
    .expect("dispatch spreadsheet search");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let search = match envelope.response {
        Response::SpreadsheetCellFind(search) => search,
        other => panic!("expected spreadsheet search, got {other:?}"),
    };
    assert_eq!(search.snapshot, imported.snapshot.to_string());
    assert_eq!(search.total, 1);
    assert_eq!(search.matches[0].worksheet, worksheet.to_string());
    assert_eq!(
        search.matches[0].cell,
        CellId::new(workbook, worksheet, 3, 1).entity().to_string()
    );

    let cell_request = SpreadsheetCellInspectRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        worksheet: worksheet.to_string(),
        row: 3,
        column: 1,
    };
    let body = serde_json::to_string(&cell_request).expect("encode cell request");
    let response = dispatch(
        &state,
        "spreadsheet_cell_inspect",
        &body,
        "http-cell".into(),
    )
    .expect("dispatch cell inspection");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let inspection = match envelope.response {
        Response::SpreadsheetCellInspect(inspection) => inspection,
        other => panic!("expected cell inspection, got {other:?}"),
    };
    assert_eq!(inspection.snapshot, imported.snapshot.to_string());
    assert_eq!(inspection.cell.as_ref().unwrap().row, 3);

    let edit = SpreadsheetEditRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        worksheet: worksheet.to_string(),
        row: 3,
        column: 1,
        value: SpreadsheetCellValue::Number("15".into()),
    };
    let body = serde_json::to_string(&edit).expect("encode edit");
    let response = dispatch(&state, "spreadsheet_edit", &body, "http-edit".into())
        .expect("dispatch spreadsheet edit");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let candidate = match envelope.response {
        Response::SpreadsheetEdit(candidate) => candidate,
        other => panic!("expected spreadsheet edit candidate, got {other:?}"),
    };
    assert_eq!(candidate.path, "budget.xlsx");
    assert_eq!(
        candidate.validation,
        SpreadsheetCandidateValidation::Pending
    );
    let range_edit = SpreadsheetRangeEditRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        worksheet: worksheet.to_string(),
        edits: vec![
            SpreadsheetCellEdit {
                row: 3,
                column: 1,
                value: SpreadsheetCellValue::Number("20".into()),
            },
            SpreadsheetCellEdit {
                row: 3,
                column: 2,
                value: SpreadsheetCellValue::Formula {
                    expression: "=A3*2".into(),
                    cached_value: None,
                },
            },
        ],
    };
    let body = serde_json::to_string(&range_edit).expect("encode range edit");
    let response = dispatch(
        &state,
        "spreadsheet_range_edit",
        &body,
        "http-range-edit".into(),
    )
    .expect("dispatch spreadsheet range edit");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let range_candidate = match envelope.response {
        Response::SpreadsheetRangeEdit(candidate) => candidate,
        other => panic!("expected spreadsheet range candidate, got {other:?}"),
    };
    assert_eq!(range_candidate.path, "budget.xlsx");
    assert_eq!(range_candidate.base_snapshot, imported.snapshot.to_string());
    let rename = SpreadsheetRenameRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        worksheet: worksheet.to_string(),
        new_name: "Plan".to_owned(),
    };
    let body = serde_json::to_string(&rename).expect("encode rename");
    let response = dispatch(&state, "spreadsheet_rename", &body, "http-rename".into())
        .expect("dispatch spreadsheet rename");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let renamed = match envelope.response {
        Response::SpreadsheetRename(candidate) => candidate,
        other => panic!("expected spreadsheet rename candidate, got {other:?}"),
    };
    assert_eq!(renamed.path, "budget.xlsx");
    assert_eq!(renamed.base_snapshot, imported.snapshot.to_string());
    assert_eq!(
        std::fs::read(&workbook_path).expect("live workbook"),
        original
    );

    drop(state);
    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}
