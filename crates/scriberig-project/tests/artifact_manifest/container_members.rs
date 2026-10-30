//! Verifies Office package parts are represented as bounded logical evidence.

use std::fs;
use std::io::{Read, Write};
use std::sync::Arc;

use scriberig_project::{ArtifactManifest, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

/// Retains exact package-part paths and digests in the canonical artifact revision.
#[test]
fn xlsx_manifest_records_exact_internal_package_members() {
    let root = super::temporary_project();
    let mut workbook = rxls::Workbook::new();
    workbook.add_sheet("Budget").write_number(0, 0, 10.0);
    let bytes = workbook.to_xlsx_checked().expect("valid XLSX package");
    fs::write(root.join("book.xlsx"), &bytes).expect("write workbook");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("open XLSX");
    let mut expected = Vec::new();
    archive
        .by_name("xl/workbook.xml")
        .expect("workbook part")
        .read_to_end(&mut expected)
        .expect("read workbook part");

    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan workbook");
    let project = scan.project.id;
    let imported = service.import(project).expect("import workbook");
    let manifest = service
        .artifacts(project)
        .expect("artifact summaries")
        .into_iter()
        .find(|manifest| manifest.logical_key == "xlsx:book.xlsx")
        .expect("XLSX artifact");
    let source = manifest
        .members
        .iter()
        .find(|member| member.project_path.as_deref() == Some("book.xlsx"))
        .expect("outer source member");
    assert_eq!(source.digest, *blake3::hash(&bytes).as_bytes());
    let workbook_part = manifest
        .members
        .iter()
        .find(|member| member.container_part.as_deref() == Some("xl/workbook.xml"))
        .expect("logical workbook part");
    assert_eq!(workbook_part.project_path, None);
    assert_eq!(workbook_part.digest, *blake3::hash(&expected).as_bytes());

    let artifact_entity = store
        .project_entities(project)
        .expect("artifact ownership")
        .into_iter()
        .find(|entity| entity.kind == "external_artifact")
        .expect("canonical artifact entity");
    let revision = store
        .resolve(imported.snapshot, artifact_entity.entity)
        .expect("resolve artifact revision")
        .expect("artifact revision");
    let revision = store
        .get(revision)
        .expect("load artifact revision")
        .expect("artifact payload");
    let stored: ArtifactManifest =
        serde_json::from_slice(revision.payload()).expect("decode artifact manifest");
    assert_eq!(
        stored.members, manifest.members,
        "canonical artifact revision retains the exact member evidence"
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Retains exact package-part paths and digests for DOCX container evidence.
#[test]
fn docx_manifest_records_exact_internal_package_members() {
    let root = super::temporary_project();
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="urn:test"><w:body><w:p/></w:body></w:document>"#)
        .expect("document XML");
    archive
        .start_file("word/styles.xml", SimpleFileOptions::default())
        .expect("styles entry");
    archive.write_all(b"<styles/>").expect("styles XML");
    archive
        .start_file("word/_rels/document.xml.rels", SimpleFileOptions::default())
        .expect("document relationships entry");
    archive
        .write_all(
            br#"<Relationships><Relationship Id="rStyles" Target="styles.xml"/></Relationships>"#,
        )
        .expect("document relationships XML");
    let bytes = archive.finish().expect("finish DOCX").into_inner();
    fs::write(root.join("report.docx"), &bytes).expect("write document");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan document");
    let manifest = service
        .artifacts(scan.project.id)
        .expect("artifact summaries")
        .into_iter()
        .find(|manifest| manifest.logical_key == "docx:report.docx")
        .expect("DOCX artifact");
    let mut expected = Vec::new();
    zip::ZipArchive::new(std::io::Cursor::new(&bytes))
        .expect("open DOCX")
        .by_name("word/document.xml")
        .expect("document part")
        .read_to_end(&mut expected)
        .expect("read document part");
    let document = manifest
        .members
        .iter()
        .find(|member| member.container_part.as_deref() == Some("word/document.xml"))
        .expect("logical document part");
    assert_eq!(document.digest, *blake3::hash(&expected).as_bytes());
    assert!(manifest.members.iter().any(|member| {
        member.container_part.as_deref() == Some("word/_rels/document.xml.rels")
    }));
    let page = service
        .artifact_members(scan.project.id, manifest.id.entity(), 1, 1)
        .expect("bounded member page");
    assert_eq!(page.members.len(), 1);
    assert_eq!(page.total, manifest.members.len());
    assert_eq!(page.next_offset, Some(2));
    assert!(page.members[0].container_part.is_some());

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Retains exact project and opaque member digests for QGZ package evidence.
#[test]
fn qgz_manifest_records_internal_package_members() {
    let root = super::temporary_project();
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    archive
        .start_file("project.qgs", SimpleFileOptions::default())
        .expect("project entry");
    archive
        .write_all(b"<qgis><projectlayers/><layer-tree-group name='root'/></qgis>")
        .expect("project XML");
    archive
        .start_file("styles/custom.qml", SimpleFileOptions::default())
        .expect("opaque style entry");
    archive
        .write_all(b"opaque QGIS style bytes")
        .expect("style bytes");
    let bytes = archive.finish().expect("finish QGZ").into_inner();
    fs::write(root.join("project.qgz"), &bytes).expect("write QGZ");

    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan QGIS project");
    let manifest = service
        .artifacts(scan.project.id)
        .expect("artifact summaries")
        .into_iter()
        .find(|manifest| manifest.logical_key == "file:project.qgz")
        .expect("QGZ artifact");
    let outer = manifest
        .members
        .iter()
        .find(|member| member.project_path.as_deref() == Some("project.qgz"))
        .expect("outer source member");
    assert_eq!(outer.digest, *blake3::hash(&bytes).as_bytes());
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("open QGZ");
    let mut style_bytes = Vec::new();
    archive
        .by_name("styles/custom.qml")
        .expect("custom style part")
        .read_to_end(&mut style_bytes)
        .expect("read custom style");
    let style = manifest
        .members
        .iter()
        .find(|member| member.container_part.as_deref() == Some("styles/custom.qml"))
        .expect("logical style part");
    assert_eq!(style.project_path, None);
    assert_eq!(style.digest, *blake3::hash(&style_bytes).as_bytes());
    assert!(
        manifest
            .members
            .iter()
            .any(|member| { member.container_part.as_deref() == Some("project.qgs") })
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
