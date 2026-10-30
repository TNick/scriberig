//! Verifies safe QGIS layer renames preserve semantic and package content.

use std::io::{Cursor, Read, Write};

use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::{import_qgis_project, rename_layer_reference, validate_layer_rename};
use scriberig_core::EntityId;

const QGS: &str = "<qgis><projectlayers><maplayer><id>layer-1</id><layername>Parcels</layername><datasource>data.gpkg</datasource></maplayer></projectlayers><layer-tree-group name='root'><layer-tree-layer id='layer-1' name='Parcels' checked='Qt::Checked'/></layer-tree-group></qgis>";

/// Updates the map layer and tree entry while retaining other QGS bytes.
#[test]
fn qgs_rename_changes_only_layer_display_names() {
    let project = EntityId::from_bytes([41; 16]);
    let original = QGS.as_bytes();
    let imported = import_qgis_project(original, project).expect("import QGS");
    let reference = imported.project.layers[0].id;
    let output = rename_layer_reference(original, project, reference, "Parcels & Lots")
        .expect("rename QGS layer");
    validate_layer_rename(original, &output).expect("validate exact QGS rename");
    let tampered = String::from_utf8(output.clone())
        .expect("rewritten QGS is UTF-8")
        .replace("data.gpkg", "other.gpkg");
    assert!(validate_layer_rename(original, tampered.as_bytes()).is_err());
    assert_eq!(
        std::str::from_utf8(&output).expect("UTF-8 QGS"),
        "<qgis><projectlayers><maplayer><id>layer-1</id><layername>Parcels &amp; Lots</layername><datasource>data.gpkg</datasource></maplayer></projectlayers><layer-tree-group name='root'><layer-tree-layer id='layer-1' name='Parcels &amp; Lots' checked='Qt::Checked'/></layer-tree-group></qgis>"
    );
}

/// Rewrites a QGZ project member and retains unrelated package payloads.
#[test]
fn qgz_rename_preserves_unrelated_members() {
    let project = EntityId::from_bytes([42; 16]);
    let archive_comment = [0x51, 0x47, 0x00, 0xFF];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.set_raw_comment(archive_comment.to_vec().into_boxed_slice());
    writer
        .start_file("project.qgs", SimpleFileOptions::default())
        .expect("QGS member");
    writer.write_all(QGS.as_bytes()).expect("write QGS");
    writer
        .start_file("styles/custom.qml", SimpleFileOptions::default())
        .expect("style member");
    writer
        .write_all(b"opaque style bytes")
        .expect("write style");
    let original = writer.finish().expect("finish QGZ").into_inner();
    let imported = import_qgis_project(&original, project).expect("import QGZ");
    let reference = imported.project.layers[0].id;
    let output =
        rename_layer_reference(&original, project, reference, "Lots").expect("rename QGZ layer");
    let mut archive = ZipArchive::new(Cursor::new(&output)).expect("open renamed QGZ");
    assert_eq!(archive.comment(), archive_comment);
    let mut style = Vec::new();
    archive
        .by_name("styles/custom.qml")
        .expect("retained style member")
        .read_to_end(&mut style)
        .expect("read style");
    assert_eq!(style, b"opaque style bytes");
    let reopened = import_qgis_project(&output, project).expect("reopen QGZ");
    assert_eq!(reopened.project.layers[0].name, "Lots");
    validate_layer_rename(&original, &output).expect("validate exact QGZ rename");
}

/// Rejects an absent semantic reference without producing project bytes.
#[test]
fn rename_rejects_unknown_layer_identity() {
    let error = rename_layer_reference(
        QGS.as_bytes(),
        EntityId::from_bytes([43; 16]),
        EntityId::from_bytes([44; 16]),
        "Unknown",
    )
    .expect_err("unknown reference must fail");
    assert!(error.contains("absent or ambiguous"));
}
