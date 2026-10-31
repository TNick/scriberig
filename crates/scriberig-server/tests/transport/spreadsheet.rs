//! Verifies spreadsheet application behavior across in-process and HTTP transports.

use scriberig_app::{
    ApplicationContext, ApplicationError, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityScope, InspectRequest, InspectResult, RuntimeCapability,
};
use scriberig_client::Client;
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{SpreadsheetCellValue, SpreadsheetRange};
use scriberig_server::{ApplicationService, InProcessTransport};

/// Serves one authorized sparse spreadsheet page for the remote boundary test.
struct SpreadsheetFixture {
    project: scriberig_core::ProjectId,
    snapshot: SnapshotId,
    worksheet: EntityId,
}

impl ApplicationService for SpreadsheetFixture {
    /// Executes the required unused entity inspection operation.
    ///
    /// # Parameters
    /// - `_context`: Trusted context, unused by this fixture.
    /// - `_request`: Entity inspection request, unused by this fixture.
    ///
    /// # Returns
    /// An unsupported-operation error for this unused route.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        _request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "inspect fixture".into(),
        })
    }

    /// Provides a fixture capability descriptor.
    ///
    /// # Returns
    /// Empty capability list because the test uses an explicit grant.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        Vec::new()
    }

    /// Returns the unsupported result for unused fixture operation lookups.
    ///
    /// # Parameters
    /// - `_operation_id`: Operation identity, unused by this fixture.
    ///
    /// # Returns
    /// An unsupported-operation error.
    ///
    /// # Errors
    /// Always returns an unsupported-operation error.
    fn operation_status(
        &self,
        _operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation status fixture".into(),
        })
    }

    /// Verifies trusted grants and returns one typed spreadsheet page.
    ///
    /// # Parameters
    /// - `context`: Trusted request context carrying the exact snapshot and grant.
    /// - `project`: Project identity supplied by the protocol request.
    /// - `worksheet`: Worksheet identity supplied by the protocol request.
    /// - `range`: Requested inclusive cell range.
    /// - `offset`: Requested zero-based page offset.
    /// - `limit`: Requested maximum page size.
    ///
    /// # Returns
    /// One typed spreadsheet page fixture.
    ///
    /// # Errors
    /// Returns unsupported-operation only if fixture behavior is extended to reject.
    fn read_spreadsheet_cells(
        &self,
        context: &ApplicationContext,
        project: &str,
        worksheet: &str,
        range: Option<SpreadsheetRange>,
        offset: usize,
        limit: usize,
    ) -> Result<scriberig_app::SpreadsheetCells, ApplicationError> {
        assert_eq!(project, self.project.to_string());
        assert_eq!(worksheet, self.worksheet.to_string());
        assert_eq!(context.snapshot, Some(self.snapshot));
        assert_eq!(range.as_ref().map(|range| range.first_row), Some(4));
        assert_eq!(offset, 2);
        assert_eq!(limit, 20);
        assert!(context.capabilities.permits(
            &CapabilityAction::Inspect,
            &CapabilityScope::Custom(format!("project:{}", self.project)),
            Some(self.snapshot),
            Some(scriberig_app::OperationKind::Inspect),
        ));
        Ok(scriberig_app::SpreadsheetCells {
            project: self.project,
            snapshot: self.snapshot,
            page: scriberig_project::SpreadsheetCellPage {
                snapshot: self.snapshot,
                worksheet: self.worksheet,
                artifact_capability: Some(scriberig_project::ArtifactCapability::ReadOnlyDerived),
                artifact_diagnostic: Some("Workbook fidelity is incomplete.".to_owned()),
                cells: vec![scriberig_domain_spreadsheet::Cell {
                    id: scriberig_domain_spreadsheet::CellId::new(
                        EntityId::from_bytes([4; 16]),
                        self.worksheet,
                        4,
                        2,
                    ),
                    worksheet: self.worksheet,
                    row: 4,
                    column: 2,
                    value: scriberig_domain_spreadsheet::CellValue::Text("parcel".into()),
                    style: None,
                }],
                total: 3,
                has_more: true,
            },
        })
    }
}

/// Verifies authorized spreadsheet reads traverse protocol, dispatcher, and client.
#[test]
fn spreadsheet_page_round_trip_preserves_snapshot_range_and_typed_cells() {
    let project = scriberig_core::ProjectId::from_bytes([5; 16]);
    let snapshot = SnapshotId::new([6; 32]);
    let worksheet = EntityId::from_bytes([7; 16]);
    let grant = Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: std::collections::BTreeSet::from([CapabilityAction::Inspect]),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let transport = InProcessTransport::new(SpreadsheetFixture {
        project,
        snapshot,
        worksheet,
    })
    .with_capabilities(CapabilityContext::with_grants([grant]));
    let page = Client::new(transport)
        .read_spreadsheet_cells(
            project.to_string(),
            snapshot.to_string(),
            worksheet.to_string(),
            Some(SpreadsheetRange {
                first_row: 4,
                first_column: 1,
                last_row: 9,
                last_column: 3,
            }),
            2,
            20,
        )
        .expect("spreadsheet page");
    assert_eq!(page.snapshot, snapshot.to_string());
    assert_eq!(page.total, 3);
    assert!(page.has_more);
    assert_eq!(
        page.artifact_diagnostic.as_deref(),
        Some("Workbook fidelity is incomplete.")
    );
    assert_eq!(page.cells[0].row, 4);
    assert_eq!(page.cells[0].column, 2);
    assert_eq!(
        page.cells[0].value,
        SpreadsheetCellValue::Text("parcel".into())
    );
}

#[allow(
    clippy::too_many_lines,
    reason = "This transport integration test verifies an imported workbook through authorized inspection, candidate creation, and client decoding."
)]
/// Verifies real imported worksheet revisions are readable through remote dispatch.
#[test]
fn real_spreadsheet_application_is_exposed_through_authorized_remote_client() {
    let root = std::env::temp_dir().join(format!("scriberig-remote-xlsx-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    let mut workbook = rxls::Workbook::new();
    workbook
        .add_sheet("Budget")
        .write_formula(2, 1, "A3*3", rxls::Cell::Number(30.0));
    let workbook_path = root.join("budget.xlsx");
    let original_bytes = workbook.to_xlsx_checked().expect("serialize XLSX");
    std::fs::write(&workbook_path, &original_bytes).expect("persist workbook");
    let database = root.with_extension("sqlite");
    let store = std::sync::Arc::new(
        scriberig_store_sqlite::SqliteStore::open(&database).expect("database"),
    );
    let projects = scriberig_project::ProjectService::new(std::sync::Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan workbook");
    let project = scan.project.id;
    let imported = projects.import(project).expect("import workbook");
    let worksheet = store
        .project_entities(project)
        .expect("worksheet ownership")
        .into_iter()
        .find(|entity| entity.kind == "spreadsheet.worksheet")
        .expect("worksheet")
        .entity;
    let backend = scriberig_app::StoreInspectionBackend::new(std::sync::Arc::clone(&store));
    let service = scriberig_app::ApplicationService::new(backend).with_project_service(projects);
    let grants = CapabilityContext::with_grants([Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: std::collections::BTreeSet::from([
            CapabilityAction::Inspect,
            CapabilityAction::Propose,
        ]),
        constraints: CapabilityConstraints::unrestricted(),
    }]);
    let client = Client::new(InProcessTransport::new(service).with_capabilities(grants));
    let page = client
        .read_spreadsheet_cells(
            project.to_string(),
            imported.snapshot.to_string(),
            worksheet.to_string(),
            Some(SpreadsheetRange {
                first_row: 3,
                first_column: 2,
                last_row: 3,
                last_column: 2,
            }),
            0,
            10,
        )
        .expect("remote semantic cell page");
    assert_eq!(page.total, 1);
    assert_eq!(page.cells[0].row, 3);
    assert_eq!(page.cells[0].column, 2);
    assert_eq!(
        page.cells[0].value,
        SpreadsheetCellValue::Formula {
            expression: "=A3*3".into(),
            cached_value: Some(Box::new(SpreadsheetCellValue::Number("30".into()))),
        }
    );
    let candidate = client
        .propose_spreadsheet_cell_edit(
            project.to_string(),
            imported.snapshot.to_string(),
            worksheet.to_string(),
            3,
            2,
            SpreadsheetCellValue::Formula {
                expression: "=A3*4".into(),
                cached_value: None,
            },
        )
        .expect("remote semantic candidate");
    assert_eq!(candidate.project, project.to_string());
    assert_eq!(candidate.base_snapshot, imported.snapshot.to_string());
    assert_eq!(candidate.path, "budget.xlsx");
    assert_eq!(
        candidate.validation,
        scriberig_protocol::SpreadsheetCandidateValidation::Pending
    );
    let rename = client
        .propose_spreadsheet_rename(
            project.to_string(),
            imported.snapshot.to_string(),
            worksheet.to_string(),
            "Plan",
        )
        .expect("remote worksheet rename candidate");
    assert_eq!(rename.project, project.to_string());
    assert_eq!(rename.base_snapshot, imported.snapshot.to_string());
    assert_eq!(rename.path, "budget.xlsx");
    assert_eq!(
        rename.validation,
        scriberig_protocol::SpreadsheetCandidateValidation::Pending
    );
    assert_eq!(
        std::fs::read(&workbook_path).expect("live workbook"),
        original_bytes
    );

    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}
