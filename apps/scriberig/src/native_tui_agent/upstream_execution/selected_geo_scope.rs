//! Grants geospatial agent tools only for a verified selected `GeoPackage` entity.

use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityId,
    CapabilityScope, OperationKind,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

const GEO_ACTIONS: [&str; 2] = ["geo.set_attribute", "geo.replace_geometry"];
const ARTIFACT_INSPECT: &str = "artifact.inspect";
const SPREADSHEET_RANGE_READ: &str = "spreadsheet.read_range";

#[path = "selected_document_scope.rs"]
mod selected_document_scope;
#[path = "selected_domain_scope.rs"]
mod selected_domain_scope;
#[path = "selected_geo_entity_scope.rs"]
mod selected_geo_entity_scope;
#[path = "selected_qgis_scope.rs"]
mod selected_qgis_scope;
#[path = "selected_spreadsheet_scope.rs"]
mod selected_spreadsheet_scope;
use selected_geo_entity_scope::{selected_layer_is_editable, selection_record};

/// Removes broad geospatial access and restores it only for a selected editable `GeoPackage`.
///
/// # Parameters
/// - `context`: Trusted branch context before tool catalog construction.
/// - `scope`: Exact semantic project selection supplied to this branch.
/// - `store`: Durable ownership and snapshot store.
/// - `projects`: Project service used to verify source ownership and editability.
///
/// # Returns
/// Branch context with geospatial tools and Propose authority scoped to the selection.
pub(super) fn for_selected_geo_entity(
    context: &ApplicationContext,
    scope: Option<&ProjectSemanticScope>,
    store: &SqliteStore,
    projects: &scriberig_project::ProjectService,
) -> ApplicationContext {
    let mut scoped = context.clone();
    remove_geo_actions(&mut scoped);
    remove_artifact_inspect(&mut scoped);
    remove_spreadsheet_range_read(&mut scoped);
    selected_spreadsheet_scope::remove_spreadsheet_edit_tools(&mut scoped);
    selected_document_scope::remove_document_edit_tools(&mut scoped);
    selected_domain_scope::remove_generic_operation(&mut scoped);
    selected_domain_scope::remove_cad_tools(&mut scoped);
    selected_qgis_scope::remove_qgis_rename(&mut scoped);
    if let Some(project) = selected_domain_scope::selected_cad_project(scope, context, store) {
        remove_project_propose(&mut scoped, project);
    }
    if selected_domain_scope::selected_editable_entity(scope, context, store, projects).is_some() {
        selected_domain_scope::grant_selected_editable_operation(
            &mut scoped,
            scope,
            store,
            projects,
        );
        selected_domain_scope::grant_selected_cad_tools(&mut scoped, scope, store, projects);
    }
    if let Some((project, snapshot)) = selected_project_scope(scope, context, store) {
        if let Ok(capability) = CapabilityId::new(ARTIFACT_INSPECT) {
            scoped.capabilities.granted.push(capability);
        }
        scoped.capabilities.grants.push(Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: [CapabilityAction::Inspect].into_iter().collect(),
            constraints: CapabilityConstraints {
                snapshot: Some(snapshot),
                operation: Some(OperationKind::Inspect),
                ..CapabilityConstraints::unrestricted()
            },
        });
    }
    if let Some((project, snapshot)) = selected_xlsx_workbook(scope, context, store) {
        if let Ok(capability) = CapabilityId::new(SPREADSHEET_RANGE_READ) {
            scoped.capabilities.granted.push(capability);
        }
        scoped.capabilities.grants.push(Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: [CapabilityAction::Custom(SPREADSHEET_RANGE_READ.to_owned())]
                .into_iter()
                .collect(),
            constraints: CapabilityConstraints {
                snapshot: Some(snapshot),
                operation: Some(OperationKind::Inspect),
                ..CapabilityConstraints::unrestricted()
            },
        });
    }
    if let Some(project) =
        selected_spreadsheet_scope::selected_spreadsheet_project(scope, context, store)
    {
        remove_project_propose(&mut scoped, project);
        selected_spreadsheet_scope::grant_selected_editable_target(
            &mut scoped,
            scope,
            store,
            projects,
        );
    }
    if let Some(project) = selected_document_scope::selected_document_project(scope, context, store)
    {
        remove_project_propose(&mut scoped, project);
        selected_document_scope::grant_selected_document_edit(&mut scoped, scope, store, projects);
    }
    if let Some(project) = selected_qgis_scope::selected_qgis_project(scope, context, store) {
        remove_project_propose(&mut scoped, project);
        selected_qgis_scope::grant_selected_qgis_rename(&mut scoped, scope, store, projects);
    }
    grant_selected_geo_entity(&mut scoped, scope, context, store);
    scoped
}

/// Grants Geo mutation only for a selected editable entity in an exact project scope.
///
/// # Parameters
/// - `scoped`: Branch context whose inherited project-wide authority was removed.
/// - `scope`: Trusted semantic project selection, if one exists.
/// - `context`: Original branch authorization context.
/// - `store`: Durable project ownership and source manifest store.
fn grant_selected_geo_entity(
    scoped: &mut ApplicationContext,
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
) {
    let Some(selection) = selection_record(scope, context, store) else {
        return;
    };
    remove_project_propose(scoped, selection.project);
    let Some(layer) = selection.layer else {
        return;
    };
    let source_is_editable = store
        .project_file_descriptors(selection.project)
        .is_ok_and(|files| {
            files.into_iter().any(|file| {
                file.relative_path == selection.path
                    && file.adapter == "geopackage"
                    && file.status == "ready"
            })
        });
    if !source_is_editable || !selected_layer_is_editable(&selection, store) {
        return;
    }
    for action in GEO_ACTIONS {
        if let Ok(capability) = CapabilityId::new(action) {
            scoped.capabilities.granted.push(capability);
        }
    }
    scoped.capabilities.grants.push(Capability {
        scope: CapabilityScope::Entity(layer),
        actions: GEO_ACTIONS
            .into_iter()
            .map(|action| CapabilityAction::Custom(action.to_owned()))
            .collect(),
        constraints: CapabilityConstraints::unrestricted(),
    });
    scoped.capabilities.grants.push(Capability {
        scope: CapabilityScope::Entity(layer),
        actions: [CapabilityAction::Propose].into_iter().collect(),
        constraints: CapabilityConstraints {
            snapshot: Some(selection.snapshot),
            operation: Some(OperationKind::DomainOperation),
            ..CapabilityConstraints::unrestricted()
        },
    });
}

/// Resolves a selected workbook only when it belongs to an editable scanned XLSX source.
///
/// # Parameters
/// - `scope`: Trusted branch selection supplied by the semantic runtime.
/// - `context`: Branch authorization context whose snapshot must match.
/// - `store`: Durable source ownership and snapshot store.
///
/// # Returns
/// Project and snapshot for a selected workbook, or `None` for other selections.
fn selected_xlsx_workbook(
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
) -> Option<(ProjectId, SnapshotId)> {
    let scope = scope?;
    let project = scope.project.parse::<ProjectId>().ok()?;
    let snapshot = scope.snapshot.parse::<SnapshotId>().ok()?;
    if context.snapshot != Some(snapshot) {
        return None;
    }
    let entity = scope.selection.as_deref()?.parse::<EntityId>().ok()?;
    let record = store.project_entity(project, entity).ok()??;
    store.resolve(snapshot, entity).ok()??;
    if record.kind != "spreadsheet.workbook"
        || !store
            .project_file_descriptors(project)
            .ok()?
            .into_iter()
            .any(|file| {
                file.relative_path == record.relative_path
                    && file.adapter == "xlsx"
                    && file.status == "ready"
            })
    {
        return None;
    }
    Some((project, snapshot))
}

/// Resolves a trusted selected semantic entity to its owning project snapshot.
///
/// - `scope`: Trusted branch selection supplied by the semantic runtime.
/// - `context`: Branch authorization context whose snapshot must match.
/// - `store`: Durable ownership and snapshot store.
///
/// The selected project and snapshot when the entity exists exactly there.
///
/// # Returns
/// Project/snapshot authority inputs, or `None` for an invalid/unselected scope.
fn selected_project_scope(
    scope: Option<&ProjectSemanticScope>,
    context: &ApplicationContext,
    store: &SqliteStore,
) -> Option<(ProjectId, SnapshotId)> {
    let scope = scope?;
    let project = scope.project.parse::<ProjectId>().ok()?;
    let snapshot = scope.snapshot.parse::<SnapshotId>().ok()?;
    if context.snapshot != Some(snapshot) {
        return None;
    }
    let entity = scope.selection.as_deref()?.parse::<EntityId>().ok()?;
    store.project_entity(project, entity).ok()??;
    store.resolve(snapshot, entity).ok()??;
    Some((project, snapshot))
}

/// Removes artifact inspection grants before restoring exact selection scope.
///
/// - `context`: Branch context being constrained.
fn remove_artifact_inspect(context: &mut ApplicationContext) {
    context
        .capabilities
        .granted
        .retain(|capability| capability.as_str() != ARTIFACT_INSPECT);
    for grant in &mut context.capabilities.grants {
        grant
            .actions
            .retain(|action| action != &CapabilityAction::Custom(ARTIFACT_INSPECT.to_owned()));
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

/// Removes spreadsheet range-read grants before restoring exact workbook selection scope.
///
/// # Parameters
/// - `context`: Branch context being constrained.
fn remove_spreadsheet_range_read(context: &mut ApplicationContext) {
    context
        .capabilities
        .granted
        .retain(|capability| capability.as_str() != SPREADSHEET_RANGE_READ);
    for grant in &mut context.capabilities.grants {
        grant.actions.retain(|action| {
            action != &CapabilityAction::Custom(SPREADSHEET_RANGE_READ.to_owned())
        });
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

/// Builds the capability-filtered catalog for one semantic branch.
///
/// # Parameters
/// - `context`: Trusted branch context before selection filtering.
/// - `scope`: Branch selection scope.
/// - `store`: Durable ownership and snapshot store.
/// - `projects`: Project service used to verify source ownership and editability.
///
/// # Returns
/// Selection-scoped context, catalog, capabilities, and visible descriptors.
pub(super) fn catalog_for_branch(
    context: &ApplicationContext,
    scope: &scriberig_agent_runtime::ChildExecutionScope,
    store: &SqliteStore,
    projects: &scriberig_project::ProjectService,
) -> (
    ApplicationContext,
    scriberig_agent_runtime::tools::ToolCatalog,
    scriberig_app::CapabilityContext,
    Vec<scriberig_agent_runtime::tools::ToolDescriptor>,
) {
    let branch_context =
        for_selected_geo_entity(context, scope.semantic_scope.as_ref(), store, projects);
    let capabilities = branch_context.capabilities.clone();
    let catalog = crate::child_runtime::inspect_catalog();
    let descriptors = catalog.descriptors_for(&capabilities);
    (branch_context, catalog, capabilities, descriptors)
}

/// Removes named Geo tool grants from both identifier and scoped grant lists.
///
/// # Parameters
/// - `context`: Branch context being constrained.
fn remove_geo_actions(context: &mut ApplicationContext) {
    context.capabilities.granted.retain(|capability| {
        !GEO_ACTIONS
            .iter()
            .any(|action| capability.as_str() == *action)
    });
    for grant in &mut context.capabilities.grants {
        grant.actions.retain(|action| {
            !GEO_ACTIONS
                .iter()
                .any(|name| action == &CapabilityAction::Custom((*name).to_owned()))
        });
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

/// Removes broad project Propose authority before adding a selected-layer grant.
///
/// # Parameters
/// - `context`: Branch context being constrained.
/// - `project`: Selected project whose broad grants are removed.
fn remove_project_propose(context: &mut ApplicationContext, project: ProjectId) {
    let project_scope = format!("project:{project}");
    for grant in &mut context.capabilities.grants {
        let project_wide = matches!(&grant.scope, CapabilityScope::Project)
            || matches!(&grant.scope, CapabilityScope::Custom(scope) if scope == &project_scope);
        if project_wide {
            grant
                .actions
                .retain(|action| action != &CapabilityAction::Propose);
        }
    }
    context
        .capabilities
        .grants
        .retain(|grant| !grant.actions.is_empty());
}

#[cfg(test)]
#[path = "selected_geo_scope_tests.rs"]
mod tests;
