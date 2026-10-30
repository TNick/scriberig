//! Grants QGIS rename tools only for an exact selected project reference.

use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityId,
    CapabilityScope, OperationKind,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

const QGIS_RENAME: &str = "qgis.rename_layer_reference";

/// Removes QGIS rename authority before exact source selection is verified.
///
/// # Parameters
/// - `context`: Branch authorization context to narrow.
pub(super) fn remove_qgis_rename(context: &mut ApplicationContext) {
    context
        .capabilities
        .granted
        .retain(|capability| capability.as_str() != QGIS_RENAME);
    for grant in &mut context.capabilities.grants {
        grant
            .actions
            .retain(|action| action != &CapabilityAction::Custom(QGIS_RENAME.to_owned()));
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

/// Restores QGIS rename authority for a parsed reference in its exact snapshot.
///
/// # Parameters
/// - `context`: Narrowed branch authorization context.
/// - `scope`: Trusted project, snapshot, and selected reference.
/// - `store`: Durable source ownership and semantic revision store.
/// - `projects`: Project service used to verify the persisted QGIS projection.
pub(super) fn grant_selected_qgis_rename(
    context: &mut ApplicationContext,
    scope: Option<&ProjectSemanticScope>,
    store: &SqliteStore,
    projects: &scriberig_project::ProjectService,
) {
    let Some((project, snapshot, reference, path)) = selected_qgis_reference(scope, context, store)
    else {
        return;
    };
    let Ok(view) = projects.qgis_project(project, &path, snapshot) else {
        return;
    };
    if !view
        .project
        .layers
        .iter()
        .any(|layer| layer.id == reference)
    {
        return;
    }
    let Ok(capability) = CapabilityId::new(QGIS_RENAME) else {
        return;
    };
    context.capabilities.granted.push(capability);
    context.capabilities.grants.push(Capability {
        scope: CapabilityScope::Entity(reference),
        actions: [CapabilityAction::Propose].into_iter().collect(),
        constraints: CapabilityConstraints {
            snapshot: Some(snapshot),
            operation: Some(OperationKind::DomainOperation),
            ..CapabilityConstraints::unrestricted()
        },
    });
}

/// Resolves the project of an exact selected QGIS reference.
///
/// # Parameters
/// - `scope`: Trusted project, snapshot, and selected semantic entity.
/// - `context`: Branch context whose snapshot must equal the supplied scope.
/// - `store`: Durable ownership and revision store.
///
/// # Returns
/// Owning project when the selected QGIS reference exists in the exact snapshot.
pub(super) fn selected_qgis_project(
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
) -> Option<ProjectId> {
    let scope = scope?;
    let project = scope.project.parse::<ProjectId>().ok()?;
    let snapshot = scope.snapshot.parse::<SnapshotId>().ok()?;
    if context.snapshot != Some(snapshot) {
        return None;
    }
    let reference = scope.selection.as_deref()?.parse::<EntityId>().ok()?;
    let record = store.project_entity(project, reference).ok()??;
    if record.kind != "qgis.layer_reference" {
        return None;
    }
    store.resolve(snapshot, reference).ok()??;
    Some(project)
}

/// Resolves a selected reference against a ready QGIS source and exact snapshot.
///
/// # Parameters
/// - `scope`: Trusted project, snapshot, and selected semantic entity.
/// - `context`: Branch context whose snapshot must equal the supplied scope.
/// - `store`: Durable ownership, source descriptor, and revision store.
///
/// # Returns
/// Exact project, snapshot, reference, and source path for a valid selection.
fn selected_qgis_reference(
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
) -> Option<(ProjectId, SnapshotId, EntityId, String)> {
    let scope = scope?;
    let project = scope.project.parse::<ProjectId>().ok()?;
    let snapshot = scope.snapshot.parse::<SnapshotId>().ok()?;
    if context.snapshot != Some(snapshot) {
        return None;
    }
    let reference = scope.selection.as_deref()?.parse::<EntityId>().ok()?;
    let record = store.project_entity(project, reference).ok()??;
    if record.kind != "qgis.layer_reference"
        || !store
            .project_file_descriptors(project)
            .ok()?
            .into_iter()
            .any(|file| {
                file.relative_path == record.relative_path
                    && file.adapter == "qgis"
                    && file.status == "ready"
            })
    {
        return None;
    }
    store.resolve(snapshot, reference).ok()??;
    Some((project, snapshot, reference, record.relative_path))
}
