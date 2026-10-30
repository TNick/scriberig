//! Grants XLSX mutation tools only for selected editable semantic targets.

use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityId,
    CapabilityScope, OperationKind,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_project::ArtifactCapability;
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

const EDIT_TOOLS: [&str; 2] = ["spreadsheet.set_cell_value", "spreadsheet.rename_sheet"];

/// Removes spreadsheet mutation tools before worksheet fidelity is verified.
///
/// # Parameters
/// - `context`: Branch authorization context to narrow.
pub(super) fn remove_spreadsheet_edit_tools(context: &mut ApplicationContext) {
    context
        .capabilities
        .granted
        .retain(|capability| !EDIT_TOOLS.iter().any(|name| capability.as_str() == *name));
    for grant in &mut context.capabilities.grants {
        grant.actions.retain(|action| {
            !EDIT_TOOLS
                .iter()
                .any(|name| action == &CapabilityAction::Custom((*name).to_owned()))
        });
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

/// Restores the matching mutation tool under exact target and snapshot authority.
///
/// # Parameters
/// - `context`: Narrowed branch authorization context.
/// - `scope`: Trusted project, snapshot, and selected worksheet or cell.
/// - `store`: Durable source ownership and revision store.
/// - `projects`: Persisted artifact fidelity service.
pub(super) fn grant_selected_editable_target(
    context: &mut ApplicationContext,
    scope: Option<&ProjectSemanticScope>,
    store: &SqliteStore,
    projects: &scriberig_project::ProjectService,
) {
    let Some((snapshot, target, operation)) =
        selected_editable_target(scope, context, store, projects)
    else {
        return;
    };
    if let Ok(capability) = CapabilityId::new(operation) {
        context.capabilities.granted.push(capability);
    }
    context.capabilities.grants.push(Capability {
        scope: CapabilityScope::Entity(target),
        actions: [CapabilityAction::Propose].into_iter().collect(),
        constraints: CapabilityConstraints {
            snapshot: Some(snapshot),
            operation: Some(OperationKind::DomainOperation),
            ..CapabilityConstraints::unrestricted()
        },
    });
}

/// Resolves the project for an exact selected spreadsheet worksheet or cell.
///
/// # Parameters
/// - `scope`: Trusted project, snapshot, and selected semantic entity.
/// - `context`: Branch context whose snapshot must match the selection.
/// - `store`: Durable ownership and revision store.
///
/// # Returns
/// Owning project when the selected target exists at the exact snapshot.
pub(super) fn selected_spreadsheet_project(
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
    if !matches!(
        record.kind.as_str(),
        "spreadsheet.worksheet" | "spreadsheet.cell"
    ) {
        return None;
    }
    store.resolve(snapshot, target).ok()??;
    Some(project)
}

/// Resolves an exact selected worksheet or cell in a ready editable XLSX artifact.
///
/// # Parameters
/// - `scope`: Trusted project, snapshot, and selected semantic entity.
/// - `context`: Branch context whose snapshot must match the selection.
/// - `store`: Durable ownership, source descriptor, and revision store.
/// - `projects`: Persisted artifact fidelity service.
///
/// # Returns
/// Project, snapshot, target, and matching tool when editability is proven.
fn selected_editable_target(
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
    projects: &scriberig_project::ProjectService,
) -> Option<(SnapshotId, EntityId, &'static str)> {
    let scope = scope?;
    let project = scope.project.parse::<ProjectId>().ok()?;
    let snapshot = scope.snapshot.parse::<SnapshotId>().ok()?;
    if context.snapshot != Some(snapshot) {
        return None;
    }
    let target = scope.selection.as_deref()?.parse::<EntityId>().ok()?;
    let record = store.project_entity(project, target).ok()??;
    let operation = match record.kind.as_str() {
        "spreadsheet.worksheet" => "spreadsheet.rename_sheet",
        "spreadsheet.cell" => "spreadsheet.set_cell_value",
        _ => return None,
    };
    store.resolve(snapshot, target).ok()??;
    let file = store
        .project_file_descriptors(project)
        .ok()?
        .into_iter()
        .find(|file| {
            file.relative_path == record.relative_path
                && file.adapter == "xlsx"
                && file.status == "ready"
        })?;
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
    editable.then_some((snapshot, target, operation))
}
