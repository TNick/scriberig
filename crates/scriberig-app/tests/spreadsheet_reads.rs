//! Verifies authorized application reads of exact-snapshot spreadsheet cells.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_agent_core::Tool;
use scriberig_app::agent_tools::{SPREADSHEET_RANGE_READ_TOOL, SpreadsheetRangeReadTool};
use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, Capability, CapabilityAction,
    CapabilityConstraints, CapabilityContext, CapabilityScope, QueryService, SpreadsheetCellQuery,
    SpreadsheetCellSearchRequest, SpreadsheetCellsQuery,
};
use scriberig_domain_spreadsheet::{CellId, CellRange};
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Creates a scoped Inspect context bound to an exact imported snapshot.
///
/// # Parameters
/// - `project`: Project covered by the Inspect grant.
/// - `snapshot`: Exact immutable snapshot granted for inspection.
///
/// # Returns
/// Authorized application context for the project snapshot.
fn inspect_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Inspect]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    }
}

/// Provides deterministic state for a large spreadsheet read scenario.
struct SpreadsheetReadFixture {
    /// Fresh temporary project directory removed after the scenario.
    root: std::path::PathBuf,
    /// Temporary SQLite database removed after the scenario.
    database: std::path::PathBuf,
    /// Shared application service with project operations enabled.
    service: ApplicationService<scriberig_app::StoreInspectionBackend<Arc<SqliteStore>>>,
    /// Project inspected by each operation.
    project: scriberig_core::ProjectId,
    /// Exact immutable imported snapshot.
    snapshot: scriberig_core::SnapshotId,
    /// Semantic worksheet identity in the project.
    worksheet: scriberig_core::EntityId,
    /// Semantic workbook identity in the project.
    workbook: scriberig_core::EntityId,
    /// Inspect context scoped to the exact snapshot.
    context: ApplicationContext,
}

/// Creates and imports a workbook with thirty thousand authored cells.
///
/// # Returns
/// Temporary project state for bounded-read and search assertions.
fn large_spreadsheet_fixture() -> SpreadsheetReadFixture {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-xlsx-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut workbook = rxls::Workbook::new();
    let budget = workbook.add_sheet("Budget");
    for row in 0..30_000 {
        budget.write_number(row, 0, f64::from(row));
    }
    fs::write(
        root.join("budget.xlsx"),
        workbook.to_xlsx_checked().expect("serialize workbook"),
    )
    .expect("write workbook");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let imported = projects.import(scan.project.id).expect("import workbook");
    let entities = store
        .project_entities(scan.project.id)
        .expect("semantic entities");
    let worksheet = entities
        .iter()
        .find(|entity| entity.kind == "spreadsheet.worksheet")
        .expect("worksheet")
        .entity;
    let workbook = entities
        .iter()
        .find(|entity| entity.kind == "spreadsheet.workbook")
        .expect("workbook")
        .entity;
    SpreadsheetReadFixture {
        root,
        database,
        service: ApplicationService::new(scriberig_app::StoreInspectionBackend::new(Arc::clone(
            &store,
        )))
        .with_project_service(projects),
        project: scan.project.id,
        snapshot: imported.snapshot,
        worksheet,
        workbook,
        context: inspect_context(scan.project.id, imported.snapshot),
    }
}

/// Verifies exact-snapshot search, workbook summary, and coordinate inspection.
///
/// - `fixture`: Imported large spreadsheet and authorized application context.
fn assert_search_and_inspection(fixture: &SpreadsheetReadFixture) {
    let search_request = SpreadsheetCellSearchRequest {
        project: fixture.project,
        snapshot: fixture.snapshot,
        workbook: fixture.workbook,
        query: "29999".into(),
        offset: 0,
        limit: 10,
    };
    assert!(matches!(
        fixture.service.find_spreadsheet_cells(
            &ApplicationContext::default().with_snapshot(fixture.snapshot),
            &search_request
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    assert!(matches!(
        fixture
            .service
            .find_spreadsheet_cells(&ApplicationContext::default(), &search_request),
        Err(ApplicationError::InvalidRequest { .. })
    ));
    let found = fixture
        .service
        .find_spreadsheet_cells(&fixture.context, &search_request)
        .expect("find exact authored workbook value");
    assert_eq!(found.snapshot, fixture.snapshot);
    assert_eq!(found.total, 1);
    assert_eq!(found.matches[0].worksheet, fixture.worksheet);
    assert_eq!(
        found.matches[0].cell,
        CellId::new(fixture.workbook, fixture.worksheet, 30_000, 1).entity()
    );
    assert!(matches!(
        fixture.service.inspect_spreadsheet_workbook(
            &ApplicationContext::default().with_snapshot(fixture.snapshot),
            fixture.project,
            fixture.workbook,
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    assert!(matches!(
        fixture.service.inspect_spreadsheet_workbook(
            &ApplicationContext::default(),
            fixture.project,
            fixture.workbook
        ),
        Err(ApplicationError::InvalidRequest { .. })
    ));
    let summary = fixture
        .service
        .inspect_spreadsheet_workbook(&fixture.context, fixture.project, fixture.workbook)
        .expect("authorized workbook summary");
    assert_eq!(summary.snapshot, fixture.snapshot);
    assert_eq!(summary.worksheets.len(), 1);
    assert_eq!(summary.worksheets[0].name, "Budget");
    assert_eq!(
        summary.worksheets[0].used_range.as_ref().unwrap().last_row,
        30_000
    );
    let cell = fixture
        .service
        .inspect_spreadsheet_cell(&fixture.context, fixture.project, fixture.worksheet, 1, 1)
        .expect("exact cell inspection");
    assert_eq!(cell.cell.as_ref().unwrap().row, 1);
    let blank = fixture
        .service
        .inspect_spreadsheet_cell(
            &fixture.context,
            fixture.project,
            fixture.worksheet,
            30_001,
            1,
        )
        .expect("sparse blank coordinate");
    assert_eq!(blank.cell, None);
}

/// Verifies sparse pages, continuation, limits, and rectangular range reads.
///
/// - `fixture`: Imported large spreadsheet and authorized application context.
fn assert_bounded_pages(fixture: &SpreadsheetReadFixture) {
    let first = fixture
        .service
        .read_spreadsheet_cells(
            &fixture.context,
            fixture.project,
            fixture.worksheet,
            None,
            0,
            500,
        )
        .expect("authorized exact-snapshot read");
    assert_eq!(first.snapshot, fixture.snapshot);
    assert_eq!(first.page.total, 30_000);
    assert_eq!(first.page.cells.len(), 500);
    assert_eq!(first.page.cells[0].row, 1);
    assert_eq!(first.page.cells[0].column, 1);
    assert!(first.page.has_more);
    let continuation = fixture
        .service
        .read_spreadsheet_cells(
            &fixture.context,
            fixture.project,
            fixture.worksheet,
            None,
            500,
            500,
        )
        .expect("bounded continuation page");
    assert_eq!(continuation.page.cells.len(), 500);
    assert_eq!(continuation.page.cells[0].row, 501);
    assert!(continuation.page.has_more);
    let final_page = fixture
        .service
        .read_spreadsheet_cells(
            &fixture.context,
            fixture.project,
            fixture.worksheet,
            None,
            29_500,
            500,
        )
        .expect("bounded final page");
    assert_eq!(final_page.page.cells.len(), 500);
    assert_eq!(final_page.page.cells[0].row, 29_501);
    assert!(!final_page.page.has_more);
    assert!(matches!(
        fixture.service.read_spreadsheet_cells(
            &fixture.context,
            fixture.project,
            fixture.worksheet,
            None,
            0,
            501
        ),
        Err(ApplicationError::InvalidRequest { .. })
    ));
    let range = CellRange {
        first_row: 100,
        first_column: 1,
        last_row: 101,
        last_column: 1,
    };
    let range_page = fixture
        .service
        .read_spreadsheet_cells(
            &fixture.context,
            fixture.project,
            fixture.worksheet,
            Some(&range),
            0,
            10,
        )
        .expect("bounded rectangular range read");
    assert_eq!(range_page.page.total, 2);
    assert_eq!(range_page.page.cells[0].row, 100);
    assert_eq!(range_page.page.cells[1].row, 101);
}

/// Verifies agent and query facades retain the shared bounded read semantics.
///
/// - `fixture`: Imported large spreadsheet and authorized application context.
fn assert_agent_and_query_facades(fixture: &SpreadsheetReadFixture) {
    let range_tool = SpreadsheetRangeReadTool::new(fixture.service.clone(), fixture.project);
    let tool_page = range_tool
        .invoke_named(
            &fixture.context,
            SPREADSHEET_RANGE_READ_TOOL,
            &serde_json::json!({
                "worksheet": fixture.worksheet.to_string(),
                "offset": 0,
                "limit": 500
            }),
        )
        .expect("authorized bounded agent range read");
    assert_eq!(tool_page.snapshot, Some(fixture.snapshot));
    assert_eq!(tool_page.handles.len(), 500);
    let tool_summary: serde_json::Value =
        serde_json::from_str(tool_page.summary.as_deref().expect("agent result summary"))
            .expect("agent summary is valid JSON");
    assert_eq!(tool_summary["total"], 30_000);
    assert_eq!(tool_summary["cells"].as_array().map(Vec::len), Some(500));
    let denied_tool_page = range_tool.invoke_named(
        &ApplicationContext::default().with_snapshot(fixture.snapshot),
        SPREADSHEET_RANGE_READ_TOOL,
        &serde_json::json!({ "worksheet": fixture.worksheet.to_string() }),
    );
    assert!(matches!(
        denied_tool_page,
        Err(scriberig_agent_core::ToolError::CapabilityDenied { .. })
    ));
    let query_page = QueryService::read_spreadsheet_cells(
        &fixture.service,
        &fixture.context,
        &SpreadsheetCellsQuery {
            project: fixture.project,
            snapshot: fixture.snapshot,
            worksheet: fixture.worksheet,
            range: Some(CellRange {
                first_row: 100,
                first_column: 1,
                last_row: 101,
                last_column: 1,
            }),
            offset: 0,
            limit: 10,
        },
    )
    .expect("query service exposes the shared spreadsheet read");
    assert_eq!(query_page.snapshot, fixture.snapshot);
    assert_eq!(query_page.page.cells.len(), 2);
    let query_cell = QueryService::inspect_spreadsheet_cell(
        &fixture.service,
        &fixture.context,
        &SpreadsheetCellQuery {
            project: fixture.project,
            snapshot: fixture.snapshot,
            worksheet: fixture.worksheet,
            row: 3,
            column: 1,
        },
    )
    .expect("query service exposes exact cell inspection");
    assert_eq!(query_cell.snapshot, fixture.snapshot);
    assert_eq!(
        query_cell.cell.as_ref().map(|cell| &cell.value),
        Some(&scriberig_domain_spreadsheet::CellValue::Number(
            "2".to_owned()
        ))
    );
}

/// Reads bounded cells/search and rejects missing grants or snapshot selectors.
#[test]
fn spreadsheet_cell_reads_are_authorized_bounded_and_snapshot_bound() {
    let fixture = large_spreadsheet_fixture();
    assert_search_and_inspection(&fixture);
    assert_bounded_pages(&fixture);
    assert_agent_and_query_facades(&fixture);
    let SpreadsheetReadFixture {
        root,
        database,
        service,
        ..
    } = fixture;
    drop(service);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
