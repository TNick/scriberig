//! Models QGIS project configuration as references to shared geospatial data.

use scriberig_core::EntityId;
use scriberig_domain_geo::CoordinateReferenceSystem;
use serde::{Deserialize, Serialize};

mod import;
mod layer_order;
mod rename;

pub use import::{QgisImport, import_qgis_project};
pub use layer_order::{reorder_project_layers, validate_project_layer_order};
pub use rename::{rename_layer_reference, validate_layer_rename};

/// Represents a QGIS project without copying its referenced feature datasets.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QgisProject {
    /// Stable project semantic identity.
    pub id: EntityId,
    /// Authored project title when present.
    pub title: Option<String>,
    /// Authored project CRS when present.
    pub crs: Option<CoordinateReferenceSystem>,
    /// Layer references in project order.
    pub layers: Vec<QgisLayerReference>,
    /// Named project groups.
    pub groups: Vec<QgisLayerGroup>,
}

/// Represents one project layer reference to a shared dataset/layer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QgisLayerReference {
    /// Stable project-reference identity.
    pub id: EntityId,
    /// Authored display name in the project.
    pub name: String,
    /// Structured dataset source reference retaining the authored value.
    pub source: QgisDataSourceReference,
    /// Authored source layer/table name.
    pub source_layer: Option<String>,
    /// Resolved Scriberig dataset identity when available.
    pub dataset: Option<EntityId>,
    /// Resolved Scriberig geospatial layer identity when available.
    pub layer: Option<EntityId>,
    /// Normalized project-relative source path when captured in the scan.
    pub resolved_path: Option<String>,
    /// Safe explanation when the external reference is unresolved.
    pub resolution_diagnostic: Option<String>,
    /// Project-specific presentation settings.
    pub presentation: QgisLayerPresentation,
}

/// Classifies an authored QGIS data source without making it canonical identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QgisDataSourceReference {
    /// Exact authored QGIS datasource text for round-trip evidence.
    pub raw: String,
    /// Classified location kind.
    pub kind: QgisDataSourceKind,
    /// Project-relative path when it can be identified safely.
    pub path: Option<String>,
    /// QGIS provider key when supplied by the project.
    pub provider: Option<String>,
}

/// Describes how a QGIS source string locates its external content.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QgisDataSourceKind {
    /// Source points to a path relative to the QGIS project location.
    ProjectRelativePath,
    /// Source contains an absolute machine path that must not become identity.
    AbsolutePath,
    /// Source is a remote network URI.
    RemoteUri,
    /// Source is a provider-specific or unrecognized URI.
    ProviderUri,
}

/// Represents project presentation independently from dataset semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QgisLayerPresentation {
    /// Whether the project enables this layer for display.
    pub visible: bool,
    /// Authored renderer/style identifier when preserved by the adapter.
    pub renderer: Option<String>,
    /// Scale range text as authored by the QGIS project.
    pub scale_range: Option<String>,
}

/// Represents a named group of project layer-reference identities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QgisLayerGroup {
    /// Stable group semantic identity.
    pub id: EntityId,
    /// Authored group name.
    pub name: String,
    /// Layer reference identities in group order.
    pub layers: Vec<EntityId>,
}

#[cfg(test)]
mod tests {
    use super::{
        QgisDataSourceKind, QgisDataSourceReference, QgisLayerPresentation, QgisLayerReference,
        QgisProject,
    };
    use scriberig_core::EntityId;

    /// Keeps a project layer reference separate from dataset feature ownership.
    #[test]
    fn project_references_shared_dataset_and_retains_presentation() {
        let dataset = EntityId::new();
        let layer = EntityId::new();
        let project = QgisProject {
            id: EntityId::new(),
            title: Some("Planning".to_owned()),
            crs: None,
            layers: vec![QgisLayerReference {
                id: EntityId::new(),
                name: "Parcels".to_owned(),
                source: QgisDataSourceReference {
                    raw: "./parcels.gpkg|layername=parcels".to_owned(),
                    kind: QgisDataSourceKind::ProjectRelativePath,
                    path: Some("./parcels.gpkg".to_owned()),
                    provider: Some("ogr".to_owned()),
                },
                source_layer: Some("parcels".to_owned()),
                dataset: Some(dataset),
                layer: Some(layer),
                resolved_path: Some("parcels.gpkg".to_owned()),
                resolution_diagnostic: None,
                presentation: QgisLayerPresentation {
                    visible: true,
                    renderer: Some("categorized".to_owned()),
                    scale_range: None,
                },
            }],
            groups: Vec::new(),
        };
        assert_eq!(project.layers[0].dataset, Some(dataset));
        assert_eq!(project.layers[0].layer, Some(layer));
        assert_ne!(project.id, dataset);
    }
}
