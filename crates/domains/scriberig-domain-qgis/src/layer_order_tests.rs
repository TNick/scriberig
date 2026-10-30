//! Verifies QGIS project layer-order writing and safe validation.

use std::io::{Read, Write};

use scriberig_core::EntityId;
use zip::write::SimpleFileOptions;

use super::{reorder_project_layers, validate_project_layer_order};
use crate::import_qgis_project;

/// Reorders a flat project tree and validates its exact semantic delta.
#[test]
fn qgs_reorder_changes_only_project_layer_order() {
    let project = EntityId::new();
    let source = flat_qgs();
    let before = import_qgis_project(source, project).expect("import QGS");
    let order = before
        .project
        .layers
        .iter()
        .rev()
        .map(|layer| layer.id)
        .collect::<Vec<_>>();
    let current_order = before
        .project
        .layers
        .iter()
        .map(|layer| layer.id)
        .collect::<Vec<_>>();
    assert!(reorder_project_layers(source, project, &current_order).is_err());

    let staged = reorder_project_layers(source, project, &order).expect("reorder layers");
    validate_project_layer_order(source, &staged).expect("validate reorder");
    let after = import_qgis_project(&staged, project).expect("reopen reordered QGS");

    assert_eq!(
        after
            .project
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>(),
        order
    );
    assert!(
        String::from_utf8(staged)
            .expect("QGS UTF-8")
            .contains("Planning")
    );
}

/// Rejects nested groups, no-op changes, and incomplete permutations before writing.
#[test]
fn reorder_rejects_unsupported_tree_and_permutation() {
    let project = EntityId::new();
    let source = br#"<qgis><projectlayers><maplayer><id>first</id><layername>First</layername><datasource>first.gpkg</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-group name="nested"><layer-tree-layer id="first" name="First"/></layer-tree-group></layer-tree-group></qgis>"#;
    let imported = import_qgis_project(source, project).expect("import QGS");

    assert!(reorder_project_layers(source, project, &[imported.project.layers[0].id]).is_err());
    assert!(reorder_project_layers(source, project, &[]).is_err());
}

/// Rewrites only the QGS member while preserving unrelated QGZ members.
#[test]
fn qgz_reorder_preserves_unrelated_package_parts() {
    let project = EntityId::new();
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file("project.qgs", SimpleFileOptions::default())
        .expect("QGS member");
    writer.write_all(flat_qgs()).expect("write QGS");
    writer
        .start_file("styles/custom.qml", SimpleFileOptions::default())
        .expect("style member");
    writer.write_all(b"opaque style data").expect("write style");
    let source = writer.finish().expect("finish QGZ").into_inner();
    let before = import_qgis_project(&source, project).expect("import QGZ");
    let order = before
        .project
        .layers
        .iter()
        .rev()
        .map(|layer| layer.id)
        .collect::<Vec<_>>();

    let reordered = reorder_project_layers(&source, project, &order).expect("reorder QGZ");
    validate_project_layer_order(&source, &reordered).expect("validate QGZ reorder");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&reordered)).expect("open QGZ");
    let mut style = Vec::new();
    archive
        .by_name("styles/custom.qml")
        .expect("retained style")
        .read_to_end(&mut style)
        .expect("read style");
    assert_eq!(style, b"opaque style data");
}

/// Returns a flat two-layer QGS fixture for order-operation tests.
///
/// # Returns
/// Exact source bytes with declaration and layer-tree order initially aligned.
fn flat_qgs() -> &'static [u8] {
    br#"<qgis><title>Planning</title><projectlayers><maplayer><id>first</id><layername>First</layername><datasource>first.gpkg</datasource></maplayer><maplayer><id>second</id><layername>Second</layername><datasource>second.gpkg</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-layer id="first" name="First"/><layer-tree-layer id="second" name="Second"/></layer-tree-group></qgis>"#
}
