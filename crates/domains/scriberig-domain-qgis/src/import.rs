//! Imports QGIS project references without owning referenced dataset content.

use parse::{project_xml, read_groups, read_layers, read_project_crs};

use scriberig_core::EntityId;

#[path = "import/parse.rs"]
mod parse;

use crate::{QgisDataSourceKind, QgisDataSourceReference, QgisProject};

/// Reports a parsed QGIS project and conservative package capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QgisImport {
    /// Project configuration and independent dataset references.
    pub project: QgisProject,
    /// Exact original QGS or QGZ byte digest.
    pub source_digest: [u8; 32],
    /// Whether project configuration can be safely rewritten.
    pub editable: bool,
    /// Safe reason for restricted capability.
    pub diagnostic: Option<String>,
}

/// Imports `.qgs` XML or a `.qgz` package into project reference semantics.
///
/// # Parameters
/// - `bytes`: Exact project file/package bytes.
/// - `project_id`: Caller-owned semantic project identity.
///
/// # Returns
/// Parsed QGIS references, source digest, and read-only capability.
///
/// Returns an error for invalid XML or an invalid QGZ package.
///
/// # Errors
/// Returns an error when project XML or its containing package is invalid.
pub fn import_qgis_project(bytes: &[u8], project_id: EntityId) -> Result<QgisImport, String> {
    let xml = project_xml(bytes)?;
    let parsed = roxmltree::Document::parse(&xml).map_err(|error| error.to_string())?;
    let title = parsed
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "title")
        .and_then(|node| node.text())
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let (layers, native_layers) = read_layers(&parsed, project_id);
    let groups = read_groups(&parsed, project_id, &native_layers);
    let crs = read_project_crs(&parsed);
    Ok(QgisImport {
        project: QgisProject {
            id: project_id,
            title,
            crs,
            layers,
            groups,
        },
        source_digest: *blake3::hash(bytes).as_bytes(),
        editable: false,
        diagnostic: Some(
            "QGIS project writing is disabled until full project preservation is proven".into(),
        ),
    })
}

/// Classifies the authored data source and retains the complete raw value.
///
/// # Parameters
/// - `raw`: QGIS datasource text, including provider options.
/// - `provider`: Optional QGIS provider key from the map layer.
///
/// # Returns
/// Structured external source reference without resolving or normalizing its path.
fn classify_source(raw: String, provider: Option<String>) -> QgisDataSourceReference {
    let path = raw.split('|').next().unwrap_or_default().to_owned();
    let lower = raw.to_ascii_lowercase();
    let kind = if lower.starts_with("http://") || lower.starts_with("https://") {
        QgisDataSourceKind::RemoteUri
    } else if raw.starts_with('/') || raw.starts_with('\\') || raw.as_bytes().get(1) == Some(&b':')
    {
        QgisDataSourceKind::AbsolutePath
    } else if !path.is_empty()
        && !lower.contains("dbname=")
        && !lower.contains("service=")
        && !path.contains('=')
        && (path.contains('/') || path.contains('.') || path.contains('\\'))
    {
        QgisDataSourceKind::ProjectRelativePath
    } else {
        QgisDataSourceKind::ProviderUri
    };
    QgisDataSourceReference {
        raw,
        kind,
        path: (kind == QgisDataSourceKind::ProjectRelativePath).then_some(path),
        provider,
    }
}

/// Derives a deterministic identity from a parent and structural key.
///
/// # Parameters
/// - `parent`: Stable project identity.
/// - `path`: Stable map-layer position key.
///
/// # Returns
/// Deterministic project layer-reference identity.
pub(super) fn derive_id(parent: EntityId, path: &str) -> EntityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"scriberig-qgis-layer-reference-v1\0");
    hasher.update(parent.as_bytes());
    hasher.update(path.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    EntityId::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use scriberig_core::EntityId;

    use zip::{ZipWriter, write::SimpleFileOptions};

    use super::{QgisDataSourceKind, import_qgis_project};

    /// Imports project references without absorbing the referenced dataset.
    #[test]
    fn qgs_import_keeps_dataset_reference_and_read_only_capability() {
        let bytes = br#"<qgis><title>Planning</title><projectCrs><spatialrefsys><authid>EPSG:4326</authid><srid>4326</srid></spatialrefsys></projectCrs><projectlayers><maplayer><id>parcel-layer</id><layername>Parcels</layername><datasource>./parcels.gpkg|layername=parcels</datasource><provider>ogr</provider><renderer-v2 type="categorized"/></maplayer><maplayer><id>road-layer</id><layername>Roads</layername><datasource>./roads.gpkg|layername=roads</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-group name="Planning"><layer-tree-layer id="parcel-layer" name="Parcels" checked="Qt::Unchecked"/><layer-tree-layer id="road-layer" name="Roads" checked="Qt::Checked"/></layer-tree-group></layer-tree-group></qgis>"#;
        let imported = import_qgis_project(bytes, EntityId::new()).expect("parse QGS");
        assert_eq!(imported.project.title.as_deref(), Some("Planning"));
        assert_eq!(imported.project.layers.len(), 2);
        assert_eq!(imported.project.layers[0].name, "Parcels");
        assert_eq!(imported.project.layers[1].name, "Roads");
        assert_eq!(
            imported.project.layers[0].source_layer.as_deref(),
            Some("parcels")
        );
        assert_eq!(
            imported.project.layers[0].source.kind,
            QgisDataSourceKind::ProjectRelativePath
        );
        assert_eq!(
            imported.project.layers[0].source.path.as_deref(),
            Some("./parcels.gpkg")
        );
        assert_eq!(
            imported.project.layers[0].source.provider.as_deref(),
            Some("ogr")
        );
        assert_eq!(
            imported.project.layers[0].source.raw,
            "./parcels.gpkg|layername=parcels"
        );
        assert_eq!(imported.project.layers[0].dataset, None);
        assert!(!imported.project.layers[0].presentation.visible);
        assert_eq!(imported.project.groups.len(), 2);
        assert_eq!(
            imported.project.groups[1].layers,
            vec![imported.project.layers[0].id, imported.project.layers[1].id]
        );
        assert_eq!(
            imported
                .project
                .crs
                .as_ref()
                .and_then(|crs| crs.authority.as_deref()),
            Some("EPSG")
        );
        assert_eq!(
            imported
                .project
                .crs
                .as_ref()
                .and_then(|crs| crs.code.as_deref()),
            Some("4326")
        );
        assert_eq!(
            imported.project.layers[0].presentation.renderer.as_deref(),
            Some("categorized")
        );
        assert!(!imported.editable);
    }

    /// Uses QGIS layer-tree order as the project presentation order.
    #[test]
    fn project_layers_follow_layer_tree_order_not_declaration_order() {
        let bytes = br#"<qgis><projectlayers><maplayer><id>first</id><layername>First</layername><datasource>first.gpkg</datasource></maplayer><maplayer><id>second</id><layername>Second</layername><datasource>second.gpkg</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-layer id="second" name="Second"/><layer-tree-layer id="first" name="First"/></layer-tree-group></qgis>"#;
        let imported = import_qgis_project(bytes, EntityId::new()).expect("parse QGS");

        assert_eq!(
            imported
                .project
                .layers
                .iter()
                .map(|layer| layer.name.as_str())
                .collect::<Vec<_>>(),
            ["Second", "First"]
        );
    }

    /// Reads a QGS project stored inside a compressed QGZ package.
    #[test]
    fn qgz_import_reads_bounded_project_member_and_retains_package_digest() {
        let xml = br"<qgis><title>Packaged</title><projectlayers><maplayer><layername>Parcels</layername><datasource>./parcels.gpkg</datasource></maplayer></projectlayers></qgis>";
        let mut archive = ZipWriter::new(std::io::Cursor::new(Vec::new()));
        archive
            .start_file("project.qgs", SimpleFileOptions::default())
            .expect("start QGS member");
        archive.write_all(xml).expect("write QGS member");
        let bytes = archive.finish().expect("finish QGZ").into_inner();
        let imported = import_qgis_project(&bytes, EntityId::new()).expect("parse QGZ");
        assert_eq!(imported.project.title.as_deref(), Some("Packaged"));
        assert_eq!(imported.project.layers.len(), 1);
        assert_eq!(imported.source_digest, *blake3::hash(&bytes).as_bytes());
        assert!(!imported.editable);
    }

    /// Classifies local, absolute, remote, and provider-specific datasource forms.
    #[test]
    fn datasource_classification_keeps_raw_values_and_external_location_kind() {
        for (raw, expected) in [
            ("data/parcels.gpkg", QgisDataSourceKind::ProjectRelativePath),
            ("/srv/data/parcels.gpkg", QgisDataSourceKind::AbsolutePath),
            (
                "https://maps.invalid/parcels",
                QgisDataSourceKind::RemoteUri,
            ),
            (
                "dbname='gis' table=parcels",
                QgisDataSourceKind::ProviderUri,
            ),
            (
                "type=xyz&url=https://tiles.invalid/{z}/{x}/{y}",
                QgisDataSourceKind::ProviderUri,
            ),
        ] {
            let source = super::classify_source(raw.to_owned(), None);
            assert_eq!(source.kind, expected);
            assert_eq!(source.raw, raw);
        }
    }
}
