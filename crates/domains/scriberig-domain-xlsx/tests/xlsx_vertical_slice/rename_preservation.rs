//! Verifies worksheet rename reference updates and loss-preserving edit gates.

use std::io::{Cursor, Read, Write};

use super::{package_member, workbook_fixture};
use scriberig_core::EntityId;
use scriberig_domain_spreadsheet::{CellValue, Formula};
use scriberig_domain_xlsx::{
    MAX_XLSX_CELL_EDITS, XlsxCellEdit, edit_cell, edit_cells, import_xlsx, validate_cell_edit,
    validate_edit, validate_package,
};
use zip::write::SimpleFileOptions;

/// Adds unknown extension markup to a worksheet without changing its package members.
///
/// # Parameters
/// - `package`: Valid XLSX bytes to extend.
///
/// # Returns
/// Package bytes with source-significant unknown worksheet XML.
fn package_with_unknown_worksheet_markup(package: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open generated package");
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).expect("read package member");
        let name = member.name().to_owned();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes).expect("read member bytes");
        if name == "xl/worksheets/sheet1.xml" {
            let xml = String::from_utf8(bytes).expect("worksheet XML is UTF-8");
            bytes = xml
                .replace(
                    "</worksheet>",
                    "<extLst xmlns:custom=\"urn:example:extension\"><ext uri=\"urn:example\"><custom:payload value=\"preserve\"/></ext></extLst></worksheet>",
                )
                .into_bytes();
        }
        output
            .start_file(name, SimpleFileOptions::default())
            .expect("copy package member");
        output.write_all(&bytes).expect("write package member");
    }
    output.finish().expect("finish package").into_inner()
}

/// Renames a worksheet while preserving formulas, names, tables, and opaque parts.
#[test]
fn sheet_rename_updates_dependent_references_and_rejects_extra_edits() {
    let original = workbook_fixture();
    let mut spreadsheet = rxls::Spreadsheet::open(&original).expect("open source workbook");
    spreadsheet
        .set_cell_formula("Notes", 0, 1, "Budget!A3", rxls::Cell::Number(10.0))
        .expect("add dependent cross-sheet formula");
    let original = spreadsheet.save().expect("save test source workbook");

    let renamed = scriberig_domain_xlsx::rename_sheet(&original, "Budget", "Plan")
        .expect("rename and rewrite dependent references");
    validate_edit(&original, &renamed).expect("validate isolated supported sheet rename");
    let imported = import_xlsx(&renamed, EntityId::new()).expect("reopen renamed workbook");
    assert_eq!(imported.worksheets[0].name, "Plan");
    assert_eq!(imported.worksheets[0].tables[0].name, "BudgetTable");
    assert!(
        imported
            .workbook
            .defined_names
            .iter()
            .any(|name| { name.name == "BudgetStart" && name.expression == "Plan!$A$3" })
    );
    assert!(
        imported
            .workbook
            .defined_names
            .iter()
            .any(|name| { name.name == "BudgetTotal" && name.expression == "Plan!$B$3" })
    );
    let notes = imported
        .worksheets
        .iter()
        .find(|worksheet| worksheet.name == "Notes")
        .expect("unrenamed worksheet remains");
    assert!(notes.cells.iter().any(|cell| {
        cell.row == 1
            && cell.column == 2
            && matches!(&cell.value, CellValue::Formula(formula) if formula.expression == "=Plan!A3")
    }));
    assert_eq!(
        package_member(&renamed, "customXml/opaque.bin"),
        b"opaque source evidence"
    );

    let extra_edit = edit_cell(
        &renamed,
        "Notes",
        1,
        1,
        &CellValue::Text("changed".to_owned()),
    )
    .expect("make unrelated additional edit");
    assert!(validate_edit(&original, &extra_edit).is_err());
    assert!(scriberig_domain_xlsx::rename_sheet(&original, "Budget", "Notes").is_err());
}

/// Verifies unknown worksheet XML is preserved or disables package edits.
#[test]
fn unknown_worksheet_markup_is_preserved_or_reported_read_only() {
    let original = package_with_unknown_worksheet_markup(&workbook_fixture());
    let imported = import_xlsx(&original, EntityId::new()).expect("read extended workbook");
    if imported.editable {
        let edited = edit_cell(
            &original,
            "Budget",
            3,
            1,
            &CellValue::Number("11".to_owned()),
        )
        .expect("editable source retains unknown worksheet markup");
        let xml = String::from_utf8(package_member(&edited, "xl/worksheets/sheet1.xml"))
            .expect("edited worksheet XML is UTF-8");
        assert!(xml.contains("urn:example:extension"));
        assert!(xml.contains("value=\"preserve\""));
    } else {
        assert!(imported.diagnostic.is_some());
        assert!(
            edit_cell(
                &original,
                "Budget",
                3,
                1,
                &CellValue::Number("11".to_owned()),
            )
            .is_err()
        );
    }
}

/// Verifies package validation does not decide whether a formula cache is correct.
#[test]
fn stale_formula_cache_does_not_fail_package_validation() {
    let original = workbook_fixture();
    let edited = edit_cell(
        &original,
        "Budget",
        3,
        2,
        &CellValue::Formula(Formula {
            expression: "=A3*3".to_owned(),
            cached_value: Some(Box::new(CellValue::Number("20".to_owned()))),
        }),
    )
    .expect("write authored formula and advisory cached value");
    validate_package(&edited).expect("stale formula cache is not a parse failure");
    validate_edit(&original, &edited).expect("formula cache is advisory metadata");
}

/// Rejects candidate edits that alter opaque package content.
#[test]
fn candidate_validation_rejects_opaque_member_changes() {
    let original = workbook_fixture();
    let mut edited = edit_cell(
        &original,
        "Budget",
        3,
        1,
        &CellValue::Number("11".to_owned()),
    )
    .expect("edit one cell");
    edited = replace_package_member(&edited, "customXml/opaque.bin", b"tampered evidence");
    let error = validate_edit(&original, &edited).expect_err("opaque member mutation is rejected");
    assert_eq!(error.code, "xlsx_candidate_invalid");
    assert!(error.message.contains("opaque or unrelated package part"));
}

/// Rejects candidate edits that exceed the supported bounded batch size.
#[test]
fn candidate_validation_rejects_edit_set_above_bound() {
    let original = workbook_fixture();
    let edits = (1..=MAX_XLSX_CELL_EDITS)
        .map(|row| XlsxCellEdit {
            row: u32::try_from(row).expect("bounded test row") + 10,
            column: 1,
            value: CellValue::Text(format!("value-{row}")),
        })
        .collect::<Vec<_>>();
    let bounded = edit_cells(&original, "Budget", &edits).expect("bounded edit batch");
    let oversized = edit_cell(
        &bounded,
        "Budget",
        u32::try_from(MAX_XLSX_CELL_EDITS).expect("bounded edit limit") + 11,
        1,
        &CellValue::Text("one more".to_owned()),
    )
    .expect("second batch can create a package beyond one validation batch");
    let error = validate_edit(&original, &oversized).expect_err("oversized edit is rejected");
    assert!(
        error
            .message
            .contains("more cells than one bounded edit set")
    );
}

/// Rejects candidate bytes that do not contain the requested cell value.
#[test]
fn candidate_validation_rejects_wrong_target_value() {
    let original = workbook_fixture();
    let edited = edit_cell(
        &original,
        "Budget",
        3,
        1,
        &CellValue::Number("11".to_owned()),
    )
    .expect("edit target cell");
    let error = validate_cell_edit(
        &original,
        &edited,
        "Budget",
        3,
        1,
        &CellValue::Number("12".to_owned()),
    )
    .expect_err("requested value mismatch is rejected");
    assert!(error.message.contains("does not match the requested"));
}

/// Replaces one ZIP member while preserving the rest of a package.
///
/// # Parameters
/// - `package`: Original package bytes.
/// - `name`: Member path to replace.
/// - `replacement`: Replacement content.
///
/// # Returns
/// Repacked XLSX bytes with the selected member replaced.
fn replace_package_member(package: &[u8], name: &str, replacement: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).expect("open package");
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).expect("read package member");
        let member_name = member.name().to_owned();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes).expect("read member bytes");
        output
            .start_file(&member_name, SimpleFileOptions::default())
            .expect("copy package header");
        output
            .write_all(if member_name == name {
                replacement
            } else {
                &bytes
            })
            .expect("write package member");
    }
    output.finish().expect("finish package").into_inner()
}
