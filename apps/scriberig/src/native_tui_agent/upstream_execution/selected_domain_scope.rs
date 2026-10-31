//! Restricts generic semantic operations to selected editable rich entities.

use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::{
    ApplicationContext, CapabilityAction, CapabilityId, CapabilityScope, OperationKind,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_project::ArtifactCapability;
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

const DOMAIN_OPERATION: &str = "domain_operation";
const CAD_TOOLS: [&str; 2] = ["cad.update_line_geometry", "cad.change_entity_layer"];

/// Removes generic semantic operation access before target verification.
///
/// # Parameters
/// - `context`: Branch authorization context to narrow.
pub(super) fn remove_generic_operation(context: &mut ApplicationContext) {
    context
        .capabilities
        .granted
        .retain(|capability| capability.as_str() != DOMAIN_OPERATION);
    for grant in &mut context.capabilities.grants {
        grant
            .actions
            .retain(|action| action != &CapabilityAction::Custom(DOMAIN_OPERATION.to_owned()));
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

/// Removes CAD mutation tools before selection and DXF fidelity are verified.
///
/// # Parameters
/// - `context`: Branch authorization context to narrow.
pub(super) fn remove_cad_tools(context: &mut ApplicationContext) {
    context
        .capabilities
        .granted
        .retain(|capability| !CAD_TOOLS.iter().any(|name| capability.as_str() == *name));
    for grant in &mut context.capabilities.grants {
        grant.actions.retain(|action| {
            !CAD_TOOLS
                .iter()
                .any(|name| action == &CapabilityAction::Custom((*name).to_owned()))
        });
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

/// Restores generic operations only for a selected target in an editable artifact.
///
/// # Parameters
/// - `context`: Narrowed branch authorization context.
/// - `scope`: Trusted project, snapshot, and selected semantic target.
/// - `store`: Durable source ownership and revision store.
/// - `projects`: Persisted artifact fidelity service.
pub(super) fn grant_selected_editable_operation(
    context: &mut ApplicationContext,
    scope: Option<&ProjectSemanticScope>,
    store: &SqliteStore,
    projects: &scriberig_project::ProjectService,
) {
    let Some((_, snapshot, target)) = selected_editable_entity(scope, context, store, projects)
    else {
        return;
    };
    if let Ok(capability) = CapabilityId::new(DOMAIN_OPERATION) {
        context.capabilities.granted.push(capability);
    }
    context.capabilities.grants.push(scriberig_app::Capability {
        scope: CapabilityScope::Entity(target),
        actions: [CapabilityAction::Custom(DOMAIN_OPERATION.to_owned())]
            .into_iter()
            .collect(),
        constraints: scriberig_app::CapabilityConstraints {
            snapshot: Some(snapshot),
            operation: Some(OperationKind::DomainOperation),
            ..scriberig_app::CapabilityConstraints::unrestricted()
        },
    });
}

/// Restores CAD tools only for an exact selected DXF entity with editable fidelity.
///
/// # Parameters
/// - `context`: Narrowed branch authorization context.
/// - `scope`: Trusted project, snapshot, and selected semantic entity.
/// - `store`: Durable entity ownership store.
/// - `projects`: Persisted artifact fidelity service.
pub(super) fn grant_selected_cad_tools(
    context: &mut ApplicationContext,
    scope: Option<&ProjectSemanticScope>,
    store: &SqliteStore,
    projects: &scriberig_project::ProjectService,
) {
    let Some((project, snapshot, target)) =
        selected_editable_entity(scope, context, store, projects)
    else {
        return;
    };
    if store
        .project_entity(project, target)
        .ok()
        .flatten()
        .is_none_or(|record| record.kind != "cad.entity")
    {
        return;
    }
    for name in CAD_TOOLS {
        if let Ok(capability) = CapabilityId::new(name) {
            context.capabilities.granted.push(capability);
        }
    }
    context.capabilities.grants.push(scriberig_app::Capability {
        scope: CapabilityScope::Entity(target),
        actions: [CapabilityAction::Propose].into_iter().collect(),
        constraints: scriberig_app::CapabilityConstraints {
            snapshot: Some(snapshot),
            operation: Some(OperationKind::DomainOperation),
            ..scriberig_app::CapabilityConstraints::unrestricted()
        },
    });
}

/// Resolves an exact selected CAD entity or drawing even when its source is read-only.
///
/// # Parameters
/// - `scope`: Trusted project, snapshot, and selected semantic entity.
/// - `context`: Branch context whose snapshot must match the selection.
/// - `store`: Durable ownership and revision store.
///
/// # Returns
/// Owning project for a selected CAD entity or drawing in the exact snapshot.
pub(super) fn selected_cad_project(
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
    let target = scope.selection.as_deref()?.parse::<EntityId>().ok()?;
    let record = store.project_entity(project, target).ok()??;
    if !matches!(record.kind.as_str(), "cad.entity" | "cad.drawing") {
        return None;
    }
    store.resolve(snapshot, target).ok()??;
    Some(project)
}

/// Resolves one exact semantic entity backed by a ready editable artifact.
///
/// # Parameters
/// - `scope`: Trusted project, snapshot, and selected semantic target.
/// - `context`: Branch context whose snapshot must exactly match the selection.
/// - `store`: Durable ownership, source descriptor, and revision store.
/// - `projects`: Persisted artifact fidelity service.
///
/// # Returns
/// Exact snapshot, target, and source path when artifact editability is proven.
pub(super) fn selected_editable_entity(
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
    projects: &scriberig_project::ProjectService,
) -> Option<(ProjectId, SnapshotId, EntityId)> {
    let scope = scope?;
    let project = scope.project.parse::<ProjectId>().ok()?;
    let snapshot = scope.snapshot.parse::<SnapshotId>().ok()?;
    if context.snapshot != Some(snapshot) {
        return None;
    }
    let target = scope.selection.as_deref()?.parse::<EntityId>().ok()?;
    let record = store.project_entity(project, target).ok()??;
    store.resolve(snapshot, target).ok()??;
    let file = store
        .project_file_descriptors(project)
        .ok()?
        .into_iter()
        .find(|file| file.relative_path == record.relative_path && file.status == "ready")?;
    let editable = projects
        .artifacts(project)
        .ok()?
        .into_iter()
        .any(|artifact| {
            artifact.members.iter().any(|member| {
                member.project_path.as_deref() == Some(record.relative_path.as_str())
                    && member.digest == file.digest
            }) && matches!(
                artifact.fidelity.capability,
                ArtifactCapability::LosslessEditable
                    | ArtifactCapability::LossAwareEditable
                    | ArtifactCapability::EditableWithOpaquePreservation
            )
        });
    editable.then_some((project, snapshot, target))
}
