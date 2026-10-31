//! Verifies sparse XLSX import and package-preserving semantic cell edits.

use std::io::{Cursor, Read, Write};

use scriberig_core::EntityId;
use scriberig_domain_spreadsheet::{CellValue, Formula};
use scriberig_domain_xlsx::{
    XlsxCellEdit, edit_cell, edit_cells, import_xlsx, validate_cell_edit, validate_cell_edits,
    validate_edit, validate_package,
};
use zip::write::SimpleFileOptions;

#[path = "xlsx_vertical_slice/rename_preservation.rs"]
mod rename_preservation;

/// Creates the deterministic workbook used by the XLSX vertical slice.
///
/// # Returns
/// XLSX bytes containing sparse cells, two sheets, a defined name, and a table.
fn workbook_fixture() -> Vec<u8> {
    let mut workbook = rxls::Workbook::new();
    let budget = workbook.add_sheet("Budget");
    budget.write_number(2, 0, 10.0);
    budget.write_formula(2, 1, "A3*2", rxls::Cell::Number(20.0));
    budget.write(2, 2, "existing");
    budget.add_table(rxls::Table::new(
        (0, 0, 2, 2),
        "BudgetTable",
        ["Budget", "Calculated", "Notes"],
    ));
    workbook.add_sheet("Notes").write(0, 0, "preserve me");
    workbook.define_name("BudgetStart", "Budget!$A$3");
    workbook.define_local_name("Budget", "BudgetTotal", "Budget!$B$3");
    let package = workbook.to_xlsx_checked().expect("valid XLSX fixture");
    package_with_opaque_member(&package)
}

/// Adds one unknown package member to exercise preservation of uninterpreted content.
///
/// # Parameters
/// - `package`: Valid source XLSX package bytes.
///
/// # Returns
/// XLSX bytes with the extra opaque member.
fn package_with_opaque_member(package: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open generated package");
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).expect("read package member");
        let name = member.name().to_owned();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes).expect("read member bytes");
        output
            .start_file(name, SimpleFileOptions::default())
            .expect("copy member header");
        output.write_all(&bytes).expect("copy member bytes");
    }
    output
        .start_file("customXml/opaque.bin", SimpleFileOptions::default())
        .expect("add opaque member");
    output
        .write_all(b"opaque source evidence")
        .expect("write opaque member");
    output.finish().expect("finish package").into_inner()
}

/// Removes required package metadata while retaining readable worksheet content.
///
/// # Parameters
/// - `package`: Valid source XLSX package bytes.
///
/// # Returns
/// XLSX bytes missing the content-type catalog.
fn package_without_content_types(package: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open generated package");
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).expect("read package member");
        if member.name() == "[Content_Types].xml" {
            continue;
        }
        let name = member.name().to_owned();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes).expect("read member bytes");
        output
            .start_file(name, SimpleFileOptions::default())
            .expect("copy package member header");
        output.write_all(&bytes).expect("copy package member bytes");
    }
    output.finish().expect("finish package").into_inner()
}

/// Reads one member from a ZIP package.
///
/// # Parameters
/// - `package`: ZIP package bytes.
/// - `name`: Exact member path to read.
///
/// # Returns
/// The member's exact bytes.
fn package_member(package: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open package");
    let mut member = archive.by_name(name).expect("find package member");
    let mut bytes = Vec::new();
    member.read_to_end(&mut bytes).expect("read package member");
    bytes
}

/// Imports a sparse workbook and edits one formula without losing opaque parts.
#[test]
fn formula_edit_reopens_and_unrelated_package_member_is_preserved() {
    let original = workbook_fixture();
    let workbook_id = EntityId::new();
    let imported = import_xlsx(&original, workbook_id).expect("import workbook");
    assert!(imported.editable);
    assert_eq!(imported.worksheets.len(), 2);
    assert_eq!(imported.workbook.defined_names[0].name, "BudgetStart");
    let budget = &imported.worksheets[0];
    let local_name = imported
        .workbook
        .defined_names
        .iter()
        .find(|name| name.name == "BudgetTotal")
        .expect("worksheet-scoped defined name");
    assert_eq!(local_name.scope, Some(budget.id));
    assert_eq!(local_name.expression, "Budget!$B$3");
    assert_eq!(budget.cells.len(), 3);
    assert_eq!(budget.tables.len(), 1);
    assert_eq!(budget.tables[0].name, "BudgetTable");
    assert_eq!(budget.tables[0].range.first_row, 1);
    assert_eq!(budget.tables[0].range.first_column, 1);
    assert_eq!(budget.tables[0].range.last_row, 3);
    assert_eq!(budget.tables[0].range.last_column, 3);
    let repeated = import_xlsx(&original, workbook_id).expect("repeat import");
    assert_eq!(
        budget.tables[0].id, repeated.worksheets[0].tables[0].id,
        "table identity must not depend on sibling order"
    );
    let (cells, has_more) = budget.cells.page(None, 0, 10).expect("bounded cells");
    assert!(!has_more);
    let formula = cells
        .iter()
        .find(|cell| cell.row == 3 && cell.column == 2)
        .expect("formula cell");
    assert_eq!(
        formula.value,
        CellValue::Formula(Formula {
            expression: "=A3*2".into(),
            cached_value: Some(Box::new(CellValue::Number("20".into()))),
        })
    );
    let references = match &formula.value {
        CellValue::Formula(formula) => formula.a1_references(),
        _ => panic!("imported cell must remain a formula"),
    };
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].start.row, 3);
    assert_eq!(references[0].start.column, 1);
    let edited = edit_cell(
        &original,
        "Budget",
        3,
        2,
        &CellValue::Formula(Formula {
            expression: "=A3*3".into(),
            cached_value: Some(Box::new(CellValue::Number("30".into()))),
        }),
    )
    .expect("package-preserving edit");
    validate_edit(&original, &edited)
        .expect("candidate preserves workbook semantics and opaque parts");
    validate_cell_edit(
        &original,
        &edited,
        "Budget",
        3,
        2,
        &CellValue::Formula(Formula {
            expression: "=A3*3".into(),
            cached_value: Some(Box::new(CellValue::Number("30".into()))),
        }),
    )
    .expect("requested formula is present at the target coordinate");
    let reopened = rxls::Workbook::open(&edited).expect("reopen edited workbook");
    assert_eq!(
        reopened.sheets[0].cell(2, 1),
        Some(&rxls::Cell::Formula {
            formula: "A3*3".into(),
            cached: Box::new(rxls::Cell::Number(30.0)),
        })
    );
    assert_eq!(
        package_member(&edited, "customXml/opaque.bin"),
        b"opaque source evidence"
    );
    assert_eq!(
        reopened.sheets[1].cell(0, 0),
        Some(&rxls::Cell::Text("preserve me".into()))
    );
}

/// Rejects invalid worksheet targets, coordinates, and typed values cleanly.
#[test]
fn cell_edit_rejects_invalid_target_coordinates_and_value_kinds() {
    let original = workbook_fixture();
    let missing = edit_cell(
        &original,
        "Missing",
        1,
        1,
        &CellValue::Text("value".to_owned()),
    )
    .expect_err("unknown worksheet must be rejected");
    assert_eq!(missing.code, "xlsx_worksheet_not_found");

    let invalid_column = edit_cell(
        &original,
        "Budget",
        1,
        16_385,
        &CellValue::Text("value".to_owned()),
    )
    .expect_err("column beyond the XLSX grid must be rejected");
    assert_eq!(invalid_column.code, "xlsx_cell_invalid");

    let invalid_row = edit_cell(
        &original,
        "Budget",
        0,
        1,
        &CellValue::Text("value".to_owned()),
    )
    .expect_err("row zero must be rejected");
    assert_eq!(invalid_row.code, "xlsx_cell_invalid");

    let invalid_formula_cache = edit_cell(
        &original,
        "Budget",
        1,
        1,
        &CellValue::Formula(Formula {
            expression: "=A1".to_owned(),
            cached_value: Some(Box::new(CellValue::Formula(Formula {
                expression: "=B1".to_owned(),
                cached_value: None,
            }))),
        }),
    )
    .expect_err("formula-valued cache must be rejected as an invalid cell kind");
    assert_eq!(invalid_formula_cache.code, "xlsx_cell_invalid");
}

/// Rewrites multiple exact cell values in one pass and rejects unrelated changes.
#[test]
fn bounded_cell_edit_set_preserves_opaque_parts_and_validates_exact_targets() {
    let source = workbook_fixture();
    let edits = [
        XlsxCellEdit {
            row: 3,
            column: 1,
            value: CellValue::Number("11".to_owned()),
        },
        XlsxCellEdit {
            row: 3,
            column: 2,
            value: CellValue::Formula(Formula {
                expression: "=A3*3".to_owned(),
                cached_value: None,
            }),
        },
    ];

    let candidate = edit_cells(&source, "Budget", &edits).expect("edit cell set");
    validate_cell_edits(&source, &candidate, "Budget", &edits).expect("validate exact cell set");
    assert_eq!(
        package_member(&source, "customXml/opaque.bin"),
        package_member(&candidate, "customXml/opaque.bin")
    );
    let imported = import_xlsx(&candidate, EntityId::new()).expect("import candidate");
    let budget = imported
        .worksheets
        .iter()
        .find(|worksheet| worksheet.name == "Budget")
        .expect("Budget worksheet");
    let (cells, has_more) = budget.cells.page(None, 0, 500).expect("read cells");
    assert!(!has_more);
    assert!(cells.iter().any(|cell| {
        cell.row == 3 && cell.column == 1 && cell.value == CellValue::Number("11".to_owned())
    }));
    assert!(cells.iter().any(|cell| {
        cell.row == 3
            && cell.column == 2
            && matches!(&cell.value, CellValue::Formula(formula) if formula.expression == "=A3*3")
    }));
    let unrelated = edit_cell(
        &source,
        "Budget",
        3,
        3,
        &CellValue::Text("unrequested".to_owned()),
    )
    .expect("edit unrelated cell");
    assert!(validate_cell_edits(&source, &unrelated, "Budget", &edits).is_err());
}

/// Verifies lossy package metadata disables writes and exposes its diagnostic.
#[test]
fn package_metadata_loss_is_reported_as_read_only() {
    let original = workbook_fixture();
    let incomplete = package_without_content_types(&original);
    assert_eq!(
        validate_package(&incomplete)
            .expect_err("missing package root is rejected")
            .code,
        "xlsx_zip_required_part_missing"
    );
    let imported = import_xlsx(&incomplete, EntityId::new()).expect("read worksheet content");
    assert!(!imported.editable);
    assert!(
        imported
            .diagnostic
            .as_deref()
            .is_some_and(|diagnostic| diagnostic.contains("PackageMetadataLoss"))
    );
}
