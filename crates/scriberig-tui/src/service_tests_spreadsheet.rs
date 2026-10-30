//! Verifies local spreadsheet reads preserve exact identities and typed formulas.

use super::*;

/// Runs the same typed spreadsheet page contract used by the remote adapter.
#[test]
fn spreadsheet_effect_returns_exact_snapshot_cell_values() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let project = scriberig_core::ProjectId::from_bytes([102; 16]);
    let snapshot = scriberig_core::SnapshotId::new([103; 32]);
    let worksheet = EntityId::from_bytes([104; 16]);
    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::InspectSpreadsheetPage {
            request_id: 5,
            project,
            snapshot,
            worksheet,
            range: Some(scriberig_domain_spreadsheet::CellRange {
                first_row: 1,
                first_column: 1,
                last_row: 10,
                last_column: 5,
            }),
            offset: 0,
            limit: 40,
        },
    );

    assert!(matches!(
        action,
        Action::SpreadsheetLoaded { request_id: 5, page }
            if page.project == project.to_string()
                && page.snapshot == snapshot.to_string()
                && page.worksheet == worksheet.to_string()
                && page.cells.len() == 1
                && matches!(
                    page.cells[0].value,
                    scriberig_protocol::SpreadsheetCellValue::Formula { ref expression, .. }
                        if expression == "=A3*2"
                )
    ));
}
