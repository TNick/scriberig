//! Builds deterministic mixed rich-artifact fixtures for onboarding tests.

use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;

use rusqlite::Connection;
use zip::write::SimpleFileOptions;

/// Creates the deterministic workbook used by rich artifact onboarding.
///
/// # Returns
/// Exact valid XLSX package bytes.
pub(crate) fn xlsx_bytes() -> Vec<u8> {
    let mut workbook = rxls::Workbook::new();
    let budget = workbook.add_sheet("Budget");
    budget.write_number(2, 0, 10.0);
    budget.write_formula(2, 1, "A3*2", rxls::Cell::Number(20.0));
    budget.write(2, 2, "existing");
    workbook.add_sheet("Notes").write(0, 0, "preserve me");
    workbook.define_name("BudgetStart", "Budget!$A$3");
    workbook.to_xlsx_checked().expect("valid XLSX")
}

/// Creates a small DOCX package with one paragraph.
///
/// # Returns
/// Exact DOCX package bytes.
pub(crate) fn docx_bytes() -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    archive
        .start_file("word/document.xml", options)
        .expect("document entry");
    archive
        .write_all(
            br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Existing site area</w:t></w:r></w:p></w:body></w:document>"#,
        )
        .expect("document XML");
    archive
        .start_file("customXml/opaque.bin", options)
        .expect("opaque custom XML member");
    archive
        .write_all(b"rich-project-opaque-docx-anchor\0")
        .expect("opaque member bytes");
    archive.finish().expect("finish DOCX").into_inner()
}

/// Creates a small DXF drawing with the CAD conformance anchors.
///
/// # Returns
/// Exact serialized DXF bytes.
pub(crate) fn dxf_bytes() -> Vec<u8> {
    let mut drawing = dxf::Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;
    drawing.header.default_drawing_units = dxf::enums::Units::Meters;
    for name in ["PARCELS", "LABELS"] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_owned(),
            ..Default::default()
        });
    }
    let mut entity =
        dxf::entities::Entity::new(dxf::entities::EntityType::Line(dxf::entities::Line {
            p1: dxf::Point::new(0.0, 0.0, 0.0),
            p2: dxf::Point::new(4.0, 0.0, 0.0),
            ..Default::default()
        }));
    "PARCELS".clone_into(&mut entity.common.layer);
    drawing.add_entity(entity);
    let mut polyline = dxf::entities::Entity::new(dxf::entities::EntityType::LwPolyline(
        dxf::entities::LwPolyline {
            vertices: vec![
                dxf::LwPolylineVertex {
                    x: 1.0,
                    y: 2.0,
                    ..Default::default()
                },
                dxf::LwPolylineVertex {
                    x: 3.0,
                    y: 2.0,
                    ..Default::default()
                },
                dxf::LwPolylineVertex {
                    x: 3.0,
                    y: 4.0,
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
    ));
    "PARCELS".clone_into(&mut polyline.common.layer);
    drawing.add_entity(polyline);
    let mut label =
        dxf::entities::Entity::new(dxf::entities::EntityType::Text(dxf::entities::Text {
            location: dxf::Point::new(1.0, 2.0, 0.0),
            value: "Parcel 124".to_owned(),
            ..Default::default()
        }));
    "LABELS".clone_into(&mut label.common.layer);
    drawing.add_entity(label);
    let mut unrelated =
        dxf::entities::Entity::new(dxf::entities::EntityType::Circle(dxf::entities::Circle {
            center: dxf::Point::new(8.0, 8.0, 0.0),
            radius: 1.0,
            ..Default::default()
        }));
    "PARCELS".clone_into(&mut unrelated.common.layer);
    drawing.add_entity(unrelated);
    let mut bytes = Vec::new();
    drawing.save(&mut bytes).expect("serialize DXF");
    normalize_generated_dxf_headers(bytes)
}

/// Replaces nondeterministic timestamps and drawing identifiers in DXF bytes.
///
/// - `bytes`: Exact DXF serializer output.
///
/// # Returns
/// Serialized drawing bytes with time and GUID fields normalized.
fn normalize_generated_dxf_headers(bytes: Vec<u8>) -> Vec<u8> {
    let serialized = String::from_utf8(bytes).expect("DXF serializer emits UTF-8");
    let mut output = String::with_capacity(serialized.len());
    let mut date_variable = false;
    let mut date_value = false;
    let mut guid_variable = false;
    let mut guid_value = false;
    for line in serialized.split_inclusive('\n') {
        if guid_value {
            output.push_str("00000000-0000-0000-0000-000000000000");
            output.push_str(if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            });
            guid_value = false;
        } else if date_value {
            output.push_str("2451545.0");
            if line.ends_with("\r\n") {
                output.push_str("\r\n");
            } else if line.ends_with('\n') {
                output.push('\n');
            }
            date_value = false;
        } else {
            output.push_str(line);
            if date_variable && line.trim() == "40" {
                date_value = true;
                date_variable = false;
            } else if guid_variable && line.trim() == "2" {
                guid_value = true;
                guid_variable = false;
            } else if matches!(
                line.trim(),
                "$TDCREATE" | "$TDUPDATE" | "$TDUCREATE" | "$TDUUPDATE"
            ) {
                date_variable = true;
            } else if matches!(line.trim(), "$FINGERPRINTGUID" | "$VERSIONGUID") {
                guid_variable = true;
            }
        }
    }
    output.into_bytes()
}

/// Creates a QGIS project referring to the `GeoPackage` independently.
///
/// # Returns
/// Exact UTF-8 QGS project bytes.
pub(crate) fn qgs_bytes() -> Vec<u8> {
    br#"<qgis><title>Planning</title><projectCrs><spatialrefsys><authid>EPSG:4326</authid><srid>4326</srid></spatialrefsys></projectCrs><projectlayers><maplayer><id>parcel-layer</id><layername>Parcels</layername><datasource>./parcels.gpkg|layername=parcels</datasource><provider>ogr</provider><renderer-v2 type="categorized"/></maplayer><maplayer><id>road-layer</id><layername>Roads</layername><datasource>./roads.shp</datasource><provider>ogr</provider></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-group name="Planning"><layer-tree-layer id="parcel-layer" name="Parcels" checked="Qt::Checked"/><layer-tree-layer id="road-layer" name="Roads" checked="Qt::Unchecked"/></layer-tree-group></layer-tree-group></qgis>"#.to_vec()
}

/// Creates a QGS project with a missing external `GeoPackage` reference.
///
/// # Returns
/// Exact UTF-8 QGS project bytes with one unresolved dataset reference.
pub(crate) fn qgs_unresolved_bytes() -> Vec<u8> {
    br"<qgis><title>Unresolved</title><projectlayers><maplayer><id>missing-layer</id><layername>Missing</layername><datasource>./missing.gpkg|layername=missing</datasource><provider>ogr</provider></maplayer></projectlayers></qgis>".to_vec()
}

/// Creates a QGZ package containing one small QGS project.
///
/// # Returns
/// Exact QGZ package bytes.
pub(crate) fn qgz_bytes() -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("project.qgs", SimpleFileOptions::default())
        .expect("start QGS package member");
    archive
        .write_all(&qgs_bytes())
        .expect("write QGS package member");
    archive.finish().expect("finish QGZ package").into_inner()
}

/// Creates one valid two-dimensional point Shapefile record for Roads.
///
/// # Returns
/// Exact `.shp` bytes with one point at the origin.
pub(crate) fn shapefile_shape_bytes() -> Vec<u8> {
    let mut bytes = vec![0; 100];
    bytes[0..4].copy_from_slice(&9994_i32.to_be_bytes());
    bytes[24..28].copy_from_slice(&64_i32.to_be_bytes());
    bytes[28..32].copy_from_slice(&1000_i32.to_le_bytes());
    bytes[32..36].copy_from_slice(&1_i32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_be_bytes());
    bytes.extend_from_slice(&10_i32.to_be_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());
    bytes.extend_from_slice(&0.0_f64.to_le_bytes());
    bytes.extend_from_slice(&0.0_f64.to_le_bytes());
    bytes
}

/// Creates one valid dBase numeric identifier for the Roads fixture.
///
/// # Returns
/// Exact `.dbf` bytes with one `road_id` value.
pub(crate) fn shapefile_dbf_bytes() -> Vec<u8> {
    let mut bytes = vec![0; 65];
    bytes[0] = 3;
    bytes[4..8].copy_from_slice(&1_u32.to_le_bytes());
    bytes[8..10].copy_from_slice(&65_u16.to_le_bytes());
    bytes[10..12].copy_from_slice(&6_u16.to_le_bytes());
    bytes[32..43].copy_from_slice(b"road_id\0\0\0\0");
    bytes[43] = b'N';
    bytes[48] = 5;
    bytes[64] = 0x0d;
    bytes.extend_from_slice(b"    42");
    bytes
}

/// Creates the one-record Shapefile index sidecar matching the Roads fixture.
///
/// # Returns
/// Exact `.shx` bytes pointing to the `.shp` record.
pub(crate) fn shapefile_index_bytes() -> Vec<u8> {
    let mut bytes = shapefile_shape_bytes()[..100].to_vec();
    bytes[24..28].copy_from_slice(&54_i32.to_be_bytes());
    bytes.extend_from_slice(&50_i32.to_be_bytes());
    bytes.extend_from_slice(&10_i32.to_be_bytes());
    bytes
}

/// Creates deterministic WKT CRS evidence for the Roads fixture.
///
/// # Returns
/// UTF-8 projection definition bytes.
pub(crate) fn shapefile_projection_bytes() -> Vec<u8> {
    br#"GEOGCS["WGS 84"]"#.to_vec()
}

/// Creates a `GeoPackage` with one layer and two bounded point features.
///
/// # Returns
/// Exact serialized `GeoPackage` database bytes.
pub(crate) fn geopackage_bytes() -> Vec<u8> {
    let connection = Connection::open_in_memory().expect("open GeoPackage fixture");
    connection
        .execute_batch(
            "CREATE TABLE gpkg_contents (
                table_name TEXT PRIMARY KEY, data_type TEXT NOT NULL,
                identifier TEXT, description TEXT DEFAULT '',
                last_change DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER
            );
            CREATE TABLE gpkg_geometry_columns (
                table_name TEXT, column_name TEXT, geometry_type_name TEXT,
                srs_id INTEGER, z TINYINT, m TINYINT
            );
            CREATE TABLE gpkg_spatial_ref_sys (
                srs_name TEXT, srs_id INTEGER PRIMARY KEY, organization TEXT,
                organization_coordsys_id INTEGER, definition TEXT, description TEXT
            );
            INSERT INTO gpkg_spatial_ref_sys VALUES
                ('WGS 84', 4326, 'EPSG', 4326, 'EPSG:4326', '');
            CREATE TABLE parcels (
                fid INTEGER PRIMARY KEY, geom BLOB, parcel_id INTEGER NOT NULL,
                area REAL, name TEXT
            );
            INSERT INTO gpkg_contents
                (table_name,data_type,identifier,last_change,min_x,min_y,max_x,max_y,srs_id)
                VALUES ('parcels','features','Parcels','2000-01-01T00:00:00.000Z',1,2,3,4,4326);
            INSERT INTO gpkg_geometry_columns
                VALUES ('parcels','geom','POINT',4326,0,0);",
        )
        .expect("create GeoPackage catalog");
    let mut geometry = b"GP\0\x01".to_vec();
    geometry.extend_from_slice(&4326_i32.to_le_bytes());
    geometry.push(1);
    geometry.extend_from_slice(&1_u32.to_le_bytes());
    geometry.extend_from_slice(&2.0_f64.to_le_bytes());
    geometry.extend_from_slice(&3.0_f64.to_le_bytes());
    let mut other_geometry = geometry.clone();
    other_geometry[13..21].copy_from_slice(&2.5_f64.to_le_bytes());
    other_geometry[21..29].copy_from_slice(&3.5_f64.to_le_bytes());
    connection
        .execute(
            "INSERT INTO parcels VALUES (124, ?1, 124, 10.5, 'Parcel 124'),
                (125, ?2, 125, 12.0, 'Parcel 125')",
            rusqlite::params![geometry, other_geometry],
        )
        .expect("insert parcel fixtures");
    connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize GeoPackage")
        .to_vec()
}

/// Creates a mixed source project with Rust and six rich artifact types.
///
/// # Returns
/// A fresh project directory containing the mixed rich-artifact fixture.
fn build_mixed_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-rich-onboarding-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    fs::write(
        root.join("source.rs"),
        "pub fn parcel_area(width: f64, height: f64) -> f64 {\n    width * height\n}\n",
    )
    .expect("persist Rust source");
    fs::write(root.join("budget.xlsx"), xlsx_bytes()).expect("persist XLSX");
    fs::write(root.join("report.docx"), docx_bytes()).expect("persist DOCX");
    fs::write(root.join("drawing.dxf"), dxf_bytes()).expect("persist DXF");
    fs::write(root.join("project.qgs"), qgs_bytes()).expect("persist QGS");
    fs::write(root.join("project.qgz"), qgz_bytes()).expect("persist QGZ");
    fs::write(root.join("unresolved.qgs"), qgs_unresolved_bytes()).expect("persist unresolved QGS");
    fs::write(root.join("parcels.gpkg"), geopackage_bytes()).expect("persist GeoPackage");
    for (path, bytes) in [
        ("roads.shp", shapefile_shape_bytes()),
        ("roads.dbf", shapefile_dbf_bytes()),
        ("roads.shx", shapefile_index_bytes()),
        ("roads.prj", shapefile_projection_bytes()),
    ] {
        fs::write(root.join(path), bytes).expect("persist Shapefile member");
    }
    root
}

/// Copies the checked-in rich-project fixture to a fresh temporary directory.
///
/// # Returns
/// A fresh project directory whose contents are copied from stable test data.
pub(crate) fn mixed_project() -> PathBuf {
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/rich-project");
    let destination = std::env::temp_dir().join(format!(
        "scriberig-rich-fixture-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&destination).expect("create copied fixture directory");
    for entry in fs::read_dir(source).expect("read checked-in fixture") {
        let entry = entry.expect("read fixture entry");
        if entry.file_type().expect("read fixture type").is_file()
            && entry.file_name() != "README.md"
        {
            fs::copy(entry.path(), destination.join(entry.file_name()))
                .expect("copy checked-in fixture file");
        }
    }
    destination
}

/// Regenerates checked-in rich fixture binaries from the Rust source builders.
///
/// - `destination`: Repository fixture directory to populate.
///
/// Unit after all current rich test artifacts are copied into the fixture.
///
/// # Errors
/// This helper panics if generation or file copying fails.
pub(crate) fn generate_mixed_fixture(destination: &std::path::Path) {
    let generated = build_mixed_project();
    fs::create_dir_all(destination).expect("create generated fixture directory");
    for entry in fs::read_dir(destination).expect("read existing fixture directory") {
        let entry = entry.expect("read existing fixture entry");
        if entry
            .file_type()
            .expect("read existing fixture type")
            .is_file()
            && entry.file_name() != "README.md"
        {
            fs::remove_file(entry.path()).expect("remove stale generated fixture file");
        }
    }
    for entry in fs::read_dir(&generated).expect("read generated fixture") {
        let entry = entry.expect("read generated fixture entry");
        if entry
            .file_type()
            .expect("read generated fixture type")
            .is_file()
        {
            fs::copy(entry.path(), destination.join(entry.file_name()))
                .expect("copy generated fixture file");
        }
    }
    fs::remove_dir_all(generated).expect("remove generated temporary fixture");
}
