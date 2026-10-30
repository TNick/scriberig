//! Grants DOCX edit tools only for selected editable semantic targets.

use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityId, CapabilityScope, OperationKind,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_project::{ArtifactCapability, ProjectService};
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

const PARAGRAPH_EDIT: &str = "document.replace_paragraph";
const TABLE_CELL_EDIT: &str = "document.update_table_cell";
const DOCUMENT_EDIT_TOOLS: [&str; 2] = [PARAGRAPH_EDIT, TABLE_CELL_EDIT];

/// Removes document mutation capabilities before exact selection is verified.
///
/// # Parameters
/// - `context`: Branch authorization context to narrow.
pub(super) fn remove_document_edit_tools(context: &mut ApplicationContext) {
    context.capabilities.granted.retain(|capability| {
        !DOCUMENT_EDIT_TOOLS
            .iter()
            .any(|name| capability.as_str() == *name)
    });
    for grant in &mut context.capabilities.grants {
        grant.actions.retain(|action| {
            !DOCUMENT_EDIT_TOOLS
                .iter()
                .any(|name| action == &CapabilityAction::Custom((*name).to_owned()))
        });
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

/// Grants one DOCX edit operation for an exact selected target and snapshot.
///
/// # Parameters
/// - `context`: Narrowed branch authorization context.
/// - `scope`: Trusted semantic selection attached to the branch.
/// - `store`: Exact entity ownership and revision store.
/// - `projects`: Project artifact fidelity service.
pub(super) fn grant_selected_document_edit(
    context: &mut ApplicationContext,
    scope: Option<&ProjectSemanticScope>,
    store: &SqliteStore,
    projects: &ProjectService,
) {
    let Some((snapshot, target, operation)) =
        selected_editable_document_target(scope, context, store, projects)
    else {
        return;
    };
    if let Ok(capability) = CapabilityId::new(operation) {
        context.capabilities.granted.push(capability);
    }
    context.capabilities.grants.push(Capability {
        scope: CapabilityScope::Entity(target),
        actions: [CapabilityAction::Propose].into_iter().collect(),
        constraints: scriberig_app::CapabilityConstraints {
            snapshot: Some(snapshot),
            operation: Some(OperationKind::DomainOperation),
            ..scriberig_app::CapabilityConstraints::unrestricted()
        },
    });
}

/// Resolves the owning project when the exact selection is a document paragraph or cell.
///
/// # Parameters
/// - `scope`: Branch-owned project, snapshot, and selected semantic entity.
/// - `context`: Request context whose snapshot must exactly match the selection.
/// - `store`: Durable semantic ownership and snapshot store.
///
/// # Returns
/// Owning project when the selected document target exists at the exact snapshot.
pub(super) fn selected_document_project(
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
        "document.paragraph" | "document.table_cell"
    ) {
        return None;
    }
    store.resolve(snapshot, target).ok()??;
    Some(project)
}

/// Resolves one exact selected paragraph or cell only when its DOCX is editable.
///
/// # Parameters
/// - `scope`: Branch-owned project, snapshot, and selected semantic entity.
/// - `context`: Request context whose snapshot must exactly match the selection.
/// - `store`: Durable semantic ownership and snapshot store.
/// - `projects`: Persisted artifact fidelity service.
///
/// # Returns
/// Exact project, snapshot, target, and matching operation when editability is proven.
fn selected_editable_document_target(
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
    projects: &ProjectService,
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
        "document.paragraph" => PARAGRAPH_EDIT,
        "document.table_cell" => TABLE_CELL_EDIT,
        _ => return None,
    };
    store.resolve(snapshot, target).ok()??;
    if !store
        .project_file_descriptors(project)
        .ok()?
        .into_iter()
        .any(|file| {
            file.relative_path == record.relative_path
                && file.adapter == "docx"
                && file.status == "ready"
        })
    {
        return None;
    }
    let editable =
        projects
            .artifacts(project)
            .ok()?
            .into_iter()
            .any(|artifact| {
                artifact.members.iter().any(|member| {
                    member.project_path.as_deref() == Some(record.relative_path.as_str())
                }) && matches!(
                    artifact.fidelity.capability,
                    ArtifactCapability::LosslessEditable
                        | ArtifactCapability::LossAwareEditable
                        | ArtifactCapability::EditableWithOpaquePreservation
                )
            });
    editable.then_some((snapshot, target, operation))
}
