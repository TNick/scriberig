//! Verifies selected spreadsheet capability scope for workbooks, worksheets, and cells.

use super::*;

/// Exposes XLSX mutation tools only for a selected editable worksheet.
#[test]
fn spreadsheet_edit_tools_require_selected_editable_worksheet_and_snapshot() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let mut workbook = rxls::Workbook::new();
    workbook.add_sheet("Budget").write_number(0, 0, 42.0);
    fs::write(
        directory.path().join("budget.xlsx"),
        workbook.to_xlsx_checked().expect("valid XLSX fixture"),
    )
    .expect("write workbook fixture");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan workbook")
        .project
        .id;
    let snapshot = projects.import(project).expect("import workbook").snapshot;
    let worksheet = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "spreadsheet.worksheet")
        .expect("spreadsheet worksheet");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([
                CapabilityAction::Propose,
                CapabilityAction::Custom("spreadsheet.set_cell_value".to_owned()),
                CapabilityAction::Custom("spreadsheet.rename_sheet".to_owned()),
            ]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    let selected = ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: worksheet.entity.to_string(),
        selection: Some(worksheet.entity.to_string()),
    };

    let scoped = for_selected_geo_entity(&context, Some(&selected), &store, &projects);
    assert_worksheet_edit_scope(&scoped, worksheet.entity, snapshot);

    let cell = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "spreadsheet.cell")
        .expect("spreadsheet cell");
    let selected_cell = ProjectSemanticScope {
        root: worksheet.entity.to_string(),
        selection: Some(cell.entity.to_string()),
        ..selected.clone()
    };
    let cell_scoped = for_selected_geo_entity(&context, Some(&selected_cell), &store, &projects);
    assert_cell_edit_scope(&cell_scoped, snapshot);

    let unselected = for_selected_geo_entity(&context, None, &store, &projects);
    assert_no_spreadsheet_edit_tools(&unselected);
}

/// Verifies worksheet selection permits only worksheet rename editing.
///
/// # Parameters
/// - `context`: Capability context filtered for the worksheet selection.
/// - `worksheet`: Exact worksheet identity.
/// - `snapshot`: Exact project snapshot.
fn assert_worksheet_edit_scope(
    context: &ApplicationContext,
    worksheet: EntityId,
    snapshot: scriberig_core::SnapshotId,
) {
    let visible = visible_tool_names(context);
    assert!(
        visible
            .iter()
            .any(|name| name == "spreadsheet.rename_sheet")
    );
    assert!(
        !visible
            .iter()
            .any(|name| name == "spreadsheet.set_cell_value")
    );
    assert!(context.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Entity(worksheet),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
    assert!(!context.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Project,
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
}

/// Verifies cell selection permits only cell-value editing for the target entity.
///
/// # Parameters
/// - `context`: Capability context filtered for the cell selection.
/// - `snapshot`: Exact project snapshot.
fn assert_cell_edit_scope(context: &ApplicationContext, snapshot: scriberig_core::SnapshotId) {
    let visible = visible_tool_names(context);
    assert!(
        visible
            .iter()
            .any(|name| name == "spreadsheet.set_cell_value")
    );
    assert!(
        !visible
            .iter()
            .any(|name| name == "spreadsheet.rename_sheet")
    );
    assert!(!context.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Entity(EntityId::new()),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
}

/// Verifies no spreadsheet edit tools are exposed without a selected target.
///
/// # Parameters
/// - `context`: Capability context filtered with no semantic selection.
fn assert_no_spreadsheet_edit_tools(context: &ApplicationContext) {
    let visible = visible_tool_names(context);
    assert!(
        !visible
            .iter()
            .any(|name| name == "spreadsheet.set_cell_value")
    );
    assert!(
        !visible
            .iter()
            .any(|name| name == "spreadsheet.rename_sheet")
    );
}

/// Lists tools available under the supplied branch capabilities.
///
/// # Parameters
/// - `context`: Application context whose capability set is inspected.
///
/// # Returns
/// Visible child tool names.
fn visible_tool_names(context: &ApplicationContext) -> Vec<String> {
    crate::child_runtime::inspect_catalog()
        .descriptors_for(&context.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect()
}
