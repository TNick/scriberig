//! Verifies remote spreadsheet reads use exact identities and typed values.

use super::*;

/// Returns the typed remote cell page without changing semantic identities.
#[test]
fn remote_spreadsheet_read_preserves_snapshot_and_formula_value() {
    let project = scriberig_core::ProjectId::from_bytes([111; 16]);
    let snapshot = SnapshotId::new([112; 32]);
    let worksheet = EntityId::from_bytes([113; 16]);
    let response = Response::SpreadsheetCells(scriberig_protocol::SpreadsheetCellsResponse {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        worksheet: worksheet.to_string(),
        artifact_capability: Some(scriberig_protocol::ArtifactEditCapability::LosslessEditable),
        artifact_diagnostic: None,
        cells: vec![scriberig_protocol::SpreadsheetCellView {
            id: "cell-remote-1".to_owned(),
            row: 4,
            column: 27,
            value: scriberig_protocol::SpreadsheetCellValue::Formula {
                expression: "=Z4+1".to_owned(),
                cached_value: Some(Box::new(scriberig_protocol::SpreadsheetCellValue::Number(
                    "9".to_owned(),
                ))),
            },
            style: None,
        }],
        total: 1,
        has_more: false,
    });
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "spreadsheet-read".to_owned(),
            response,
        },
    }));

    let page = adapter
        .inspect_spreadsheet_cells(
            &ApplicationContext::empty(),
            &scriberig_app::SpreadsheetCellsQuery {
                project,
                snapshot,
                worksheet,
                range: Some(scriberig_domain_spreadsheet::CellRange {
                    first_row: 1,
                    first_column: 1,
                    last_row: 20,
                    last_column: 30,
                }),
                offset: 0,
                limit: 40,
            },
        )
        .expect("remote page");

    assert_eq!(page.project, project.to_string());
    assert_eq!(page.snapshot, snapshot.to_string());
    assert_eq!(page.worksheet, worksheet.to_string());
    assert_eq!(
        page.artifact_capability,
        Some(scriberig_protocol::ArtifactEditCapability::LosslessEditable)
    );
    assert!(matches!(
        &page.cells[0].value,
        scriberig_protocol::SpreadsheetCellValue::Formula { expression, cached_value }
            if expression == "=Z4+1"
                && matches!(cached_value.as_deref(), Some(scriberig_protocol::SpreadsheetCellValue::Number(value)) if value == "9")
    ));
}

/// Reads one exact remote coordinate and preserves an explicit sparse blank.
#[test]
fn remote_spreadsheet_cell_inspection_keeps_coordinate_and_blank_semantics() {
    let project = scriberig_core::ProjectId::from_bytes([121; 16]);
    let snapshot = SnapshotId::new([122; 32]);
    let worksheet = EntityId::from_bytes([123; 16]);
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "spreadsheet-cell".to_owned(),
            response: Response::SpreadsheetCellInspect(
                scriberig_protocol::SpreadsheetCellInspectResponse {
                    project: project.to_string(),
                    snapshot: snapshot.to_string(),
                    worksheet: worksheet.to_string(),
                    row: 7,
                    column: 4,
                    cell: None,
                },
            ),
        },
    }));

    let cell = adapter
        .inspect_spreadsheet_cell(
            &ApplicationContext::empty(),
            &scriberig_app::SpreadsheetCellQuery {
                project,
                snapshot,
                worksheet,
                row: 7,
                column: 4,
            },
        )
        .expect("remote coordinate inspection");

    assert_eq!(cell.project, project.to_string());
    assert_eq!(cell.snapshot, snapshot.to_string());
    assert_eq!(cell.worksheet, worksheet.to_string());
    assert_eq!((cell.row, cell.column), (7, 4));
    assert!(cell.cell.is_none());
}

/// Maps a remote formula edit response into the shared review candidate model.
#[test]
fn remote_spreadsheet_edit_preserves_candidate_and_validation_identity() {
    let project = scriberig_core::ProjectId::from_bytes([131; 16]);
    let snapshot = SnapshotId::new([132; 32]);
    let worksheet = EntityId::from_bytes([133; 16]);
    let adapter = RemoteAdapter::new(Client::new(FixtureTransport {
        response: ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: "spreadsheet-edit".to_owned(),
            response: Response::SpreadsheetEdit(scriberig_protocol::SpreadsheetEditResponse {
                project: project.to_string(),
                candidate_id: "candidate-remote-1".to_owned(),
                base_snapshot: snapshot.to_string(),
                path: "budget.xlsx".to_owned(),
                validation: scriberig_protocol::SpreadsheetCandidateValidation::Pending,
            }),
        },
    }));

    let candidate = adapter
        .propose_spreadsheet_edit(
            &ApplicationContext::empty(),
            &scriberig_app::SpreadsheetCellEditQuery {
                project,
                snapshot,
                worksheet,
                row: 3,
                column: 2,
                value: scriberig_domain_spreadsheet::CellValue::Formula(
                    scriberig_domain_spreadsheet::Formula {
                        expression: "=A1*2".to_owned(),
                        cached_value: None,
                    },
                ),
            },
        )
        .expect("remote candidate");

    assert_eq!(candidate.id, "candidate-remote-1");
    assert_eq!(candidate.project, project);
    assert_eq!(candidate.base_snapshot, snapshot);
    assert_eq!(candidate.path, "budget.xlsx");
    assert_eq!(
        candidate.validation,
        scriberig_project::CandidateValidation::Pending
    );
}
