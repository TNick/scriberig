//! Resolves exact selected `GeoPackage` entities and writable layer targets.

use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::ApplicationContext;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_geo::GeoPackageLayerInfo;
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Resolves a selection only when its exact entity exists in the exact snapshot.
///
/// # Parameters
/// - `scope`: Exact project selection supplied to the branch.
/// - `context`: Branch request snapshot.
/// - `store`: Durable ownership and revision store.
///
/// # Returns
/// Project, source path, and snapshot when the selection names Geo semantic data.
pub(super) fn selection_record(
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
) -> Option<SelectedGeoEntity> {
    let scope = scope?;
    let project = scope.project.parse::<ProjectId>().ok()?;
    let snapshot = scope.snapshot.parse::<SnapshotId>().ok()?;
    if context.snapshot != Some(snapshot) {
        return None;
    }
    let entity = scope.selection.as_deref()?.parse::<EntityId>().ok()?;
    let record = store.project_entity(project, entity).ok()??;
    if !matches!(
        record.kind.as_str(),
        "geo.dataset" | "geo.layer" | "geo.feature"
    ) {
        return None;
    }
    let revision = store.resolve(snapshot, entity).ok()??;
    let payload = store.get(revision).ok()??;
    let layer = match record.kind.as_str() {
        "geo.layer" => Some(entity),
        "geo.feature" => serde_json::from_slice::<scriberig_domain_geo::Feature>(payload.payload())
            .ok()
            .map(|feature| feature.layer),
        _ => None,
    };
    Some(SelectedGeoEntity {
        project,
        snapshot,
        path: record.relative_path,
        layer,
    })
}

/// Confirms the selected semantic layer has a safe `GeoPackage` writer capability.
///
/// # Parameters
/// - `selection`: Exact project, source path, snapshot, and selected layer.
/// - `store`: Durable entity ownership and revision store.
///
/// # Returns
/// `true` when the exact selected layer is owned by the selected source and editable.
pub(super) fn selected_layer_is_editable(
    selection: &SelectedGeoEntity,
    store: &SqliteStore,
) -> bool {
    let Some(layer) = selection.layer else {
        return false;
    };
    let Ok(Some(record)) = store.project_entity(selection.project, layer) else {
        return false;
    };
    if record.kind != "geo.layer" || record.relative_path != selection.path {
        return false;
    }
    let Ok(Some(revision)) = store.resolve(selection.snapshot, layer) else {
        return false;
    };
    let Ok(Some(revision)) = store.get(revision) else {
        return false;
    };
    serde_json::from_slice::<GeoPackageLayerInfo>(revision.payload())
        .is_ok_and(|info| info.editable)
}

/// Identifies a selected `GeoPackage` entity's exact project source and snapshot.
pub(super) struct SelectedGeoEntity {
    /// Project owning the selected semantic entity.
    pub(super) project: ProjectId,
    /// Exact selected semantic snapshot.
    pub(super) snapshot: SnapshotId,
    /// Project-relative source path.
    pub(super) path: String,
    /// Exact semantic layer authorized for edits, when selection resolves one.
    pub(super) layer: Option<EntityId>,
}
