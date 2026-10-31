//! Defines provider-neutral QGIS project and layer-reference read contracts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests one exact-snapshot QGIS project semantic view.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QgisProjectRequest {
    /// Stable Scriberig project identity.
    pub project: String,
    /// Exact project-relative `.qgs` or `.qgz` path.
    pub path: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
}

/// Returns bounded QGIS project structure and resolved shared `GeoLayer` identities.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QgisProjectResponse {
    /// Stable owning Scriberig project identity.
    pub project: String,
    /// Exact project-relative source path from the request.
    pub path: String,
    /// Exact snapshot containing these references.
    pub snapshot: String,
    /// Exact immutable semantic revision of the QGIS project model.
    pub revision: String,
    /// Stable QGIS project semantic identity.
    pub id: String,
    /// Authored QGIS project title when present.
    pub title: Option<String>,
    /// Explicit project CRS when present.
    pub crs: Option<crate::GeoCrsView>,
    /// Ordered project layer references with optional resolved semantic links.
    pub layers: Vec<QgisLayerReferenceView>,
    /// Project groups and their ordered reference identities.
    pub groups: Vec<QgisLayerGroupView>,
}

/// Requests an exact-snapshot QGIS layer-reference display-name candidate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QgisLayerRenameRequest {
    /// Stable Scriberig project identity.
    pub project: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
    /// Stable semantic QGIS layer-reference identity.
    pub reference: String,
    /// New authored display name.
    pub new_name: String,
}

/// Returns the shared review handle for one QGIS layer-reference rename.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QgisLayerRenameResponse {
    /// Stable project candidate identity.
    pub candidate: String,
    /// Stable owning project identity.
    pub project: String,
    /// Exact base semantic snapshot.
    pub base_snapshot: String,
    /// Stable semantic QGIS layer-reference identity.
    pub reference: String,
    /// Exact project-relative source path.
    pub path: String,
}

/// Requests a complete exact-snapshot QGIS project layer-order candidate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QgisLayerOrderRequest {
    /// Stable Scriberig project identity.
    pub project: String,
    /// Exact immutable semantic snapshot identity.
    pub snapshot: String,
    /// Exact project-relative `.qgs` or `.qgz` path.
    pub path: String,
    /// Complete ordered list of semantic layer-reference identities.
    pub ordered_references: Vec<String>,
}

/// Returns the shared review handle for one QGIS project layer-order edit.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QgisLayerOrderResponse {
    /// Stable project candidate identity.
    pub candidate: String,
    /// Stable owning project identity.
    pub project: String,
    /// Exact base semantic snapshot.
    pub base_snapshot: String,
    /// Exact project-relative source path.
    pub path: String,
}

/// Describes one QGIS project reference without copying linked dataset content.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QgisLayerReferenceView {
    /// Stable semantic QGIS reference identity.
    pub id: String,
    /// Authored display name.
    pub name: String,
    /// Exact authored datasource string retained as evidence.
    pub data_source: String,
    /// Project-relative path when the reference uses one.
    pub path: Option<String>,
    /// Normalized path to the scanned source artifact when it is project-relative.
    pub resolved_path: Option<String>,
    /// Authored source layer/table name.
    pub source_layer: Option<String>,
    /// Resolved shared semantic dataset identity.
    pub dataset: Option<String>,
    /// Resolved shared semantic geospatial layer identity.
    pub layer: Option<String>,
    /// Safe explanation when resolution failed.
    pub resolution_diagnostic: Option<String>,
    /// Authored display visibility.
    pub visible: bool,
    /// Authored renderer kind when supported.
    pub renderer: Option<String>,
    /// Authored scale range when present.
    pub scale_range: Option<String>,
}

/// Describes one named QGIS layer group.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QgisLayerGroupView {
    /// Stable semantic group identity.
    pub id: String,
    /// Authored group name.
    pub name: String,
    /// Ordered QGIS reference identities in the group.
    pub layers: Vec<String>,
}
