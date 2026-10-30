//! Verifies exact worksheet selection and bounded range command behavior.

use super::*;

/// Requests a bounded worksheet range under the selected exact snapshot.
#[test]
fn spreadsheet_command_uses_selected_project_worksheet_and_snapshot() {
    let project = scriberig_core::ProjectId::from_bytes([11; 16]);
    let worksheet = scriberig_core::EntityId::from_bytes([12; 16]);
    let snapshot = scriberig_core::SnapshotId::new([13; 32]);
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root: worksheet,
            entity: worksheet,
            snapshot,
        }),
        selection: Some(crate::selection::EntitySelection {
            entity: worksheet,
            revision: None,
            snapshot: Some(snapshot),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/spreadsheet B2:D9 40".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectSpreadsheetPage {
            project: requested_project,
            snapshot: requested_snapshot,
            worksheet: requested_worksheet,
            range: Some(range),
            offset: 40,
            limit: 40,
            ..
        }] if *requested_project == project
            && *requested_snapshot == snapshot
            && *requested_worksheet == worksheet
            && range.first_row == 2
            && range.first_column == 2
            && range.last_row == 9
            && range.last_column == 4
    ));
    assert!(matches!(
        state.spreadsheet_page,
        crate::inspector::LoadState::Loading { .. }
    ));
}

/// Rejects worksheet commands without exact snapshot selection.
#[test]
fn spreadsheet_command_requires_exact_snapshot_selection() {
    let mut state = TuiState::default();
    state.conversation_composer.draft = "/spreadsheet A1:B2".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state.conversation_composer.error.as_deref(),
        Some("Select a worksheet at an exact snapshot first.")
    );
}

/// Requests individual cell inspection using the exact selected worksheet context.
#[test]
fn spreadsheet_cell_command_targets_the_selected_cell_coordinate() {
    let project = scriberig_core::ProjectId::from_bytes([21; 16]);
    let worksheet = scriberig_core::EntityId::from_bytes([22; 16]);
    let snapshot = scriberig_core::SnapshotId::new([23; 32]);
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root: worksheet,
            entity: worksheet,
            snapshot,
        }),
        selection: Some(crate::selection::EntitySelection {
            entity: worksheet,
            revision: None,
            snapshot: Some(snapshot),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/spreadsheet-cell AB12".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectSpreadsheetCell {
            request_id: _,
            request
        }] if request.project == project
            && request.snapshot == snapshot
            && request.worksheet == worksheet
            && request.row == 12
            && request.column == 28
    ));
}

/// Applies an exact cell result as semantic selection without losing worksheet context.
#[test]
fn spreadsheet_cell_result_selects_entity_and_retains_worksheet() {
    let cell = scriberig_core::EntityId::from_bytes([31; 16]);
    let worksheet = scriberig_core::EntityId::from_bytes([32; 16]);
    let snapshot = scriberig_core::SnapshotId::new([33; 32]);
    let mut state = TuiState {
        spreadsheet_cell: crate::inspector::LoadState::Loading { request_id: 9 },
        spreadsheet_worksheet: Some(worksheet),
        ..TuiState::default()
    };

    reduce(
        &mut state,
        Action::SpreadsheetCellLoaded {
            request_id: 9,
            cell: scriberig_protocol::SpreadsheetCellInspectResponse {
                project: "project-1".to_owned(),
                snapshot: snapshot.to_string(),
                worksheet: worksheet.to_string(),
                row: 2,
                column: 1,
                cell: Some(scriberig_protocol::SpreadsheetCellView {
                    id: cell.to_string(),
                    row: 2,
                    column: 1,
                    value: scriberig_protocol::SpreadsheetCellValue::Text("value".to_owned()),
                    style: None,
                }),
            },
        },
    );

    assert_eq!(
        state.selection,
        Some(crate::selection::EntitySelection {
            entity: cell,
            revision: None,
            snapshot: Some(snapshot),
        })
    );
    assert_eq!(state.spreadsheet_worksheet, Some(worksheet));
}

/// Routes formula commands through exact-snapshot shared candidate creation.
#[test]
fn spreadsheet_formula_command_requests_candidate_not_materialization() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/rich-artifact/spreadsheet-cell-target.json"
    ))
    .expect("shared web/TUI spreadsheet fixture");
    let row = u32::try_from(fixture["row"].as_u64().expect("fixture row"))
        .expect("fixture row fits spreadsheet coordinates");
    let column = u32::try_from(fixture["column"].as_u64().expect("fixture column"))
        .expect("fixture column fits spreadsheet coordinates");
    let expected_formula = fixture["value"]["expression"]
        .as_str()
        .expect("fixture formula");
    let project = scriberig_core::ProjectId::from_bytes([41; 16]);
    let worksheet = scriberig_core::EntityId::from_bytes([42; 16]);
    let snapshot = scriberig_core::SnapshotId::new([43; 32]);
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root: worksheet,
            entity: worksheet,
            snapshot,
        }),
        selection: Some(crate::selection::EntitySelection {
            entity: worksheet,
            revision: None,
            snapshot: Some(snapshot),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = format!("/spreadsheet-formula B{row} {expected_formula}");

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(
        effects.as_slice(),
        [Effect::ProposeSpreadsheetEdit {
            request_id: _,
            request
        }] if request.project == project
            && request.snapshot == snapshot
            && request.worksheet == worksheet
            && request.row == row
            && request.column == column
            && matches!(
                request.value,
                scriberig_domain_spreadsheet::CellValue::Formula(ref formula)
                    if formula.expression == expected_formula
            )
    ));
    assert!(matches!(
        state.spreadsheet_edit,
        crate::inspector::LoadState::Loading { .. }
    ));
}

/// Opens the standard review effect after an edit candidate is created.
#[test]
fn spreadsheet_candidate_proposal_enters_shared_review() {
    let snapshot = scriberig_core::SnapshotId::new([51; 32]);
    let mut state = TuiState {
        spreadsheet_edit: crate::inspector::LoadState::Loading { request_id: 8 },
        next_request_id: 9,
        ..TuiState::default()
    };

    let effects = reduce(
        &mut state,
        Action::SpreadsheetEditProposed {
            request_id: 8,
            candidate: scriberig_app::SpreadsheetEditCandidate {
                id: "candidate-1".to_owned(),
                project: scriberig_core::ProjectId::from_bytes([52; 16]),
                base_snapshot: snapshot,
                path: "budget.xlsx".to_owned(),
                validation: scriberig_project::CandidateValidation::Pending,
            },
        },
    );

    assert!(matches!(
        effects.as_slice(),
        [Effect::Apply {
            request_id: 9,
            request
        }] if request.change == "candidate-1" && request.base_snapshot == snapshot
    ));
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Changes);
}
