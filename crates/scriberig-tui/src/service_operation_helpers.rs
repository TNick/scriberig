//! Implements project-aware operation adapter helpers.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_app::RuntimeCapability;

#[path = "service_operation_helpers/projected.rs"]
mod projected;

/// Inspects one entity and supplies a project label when needed.
pub(super) fn inspect_entity<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    context: &ApplicationContext,
    entity: EntityId,
    snapshot: Option<SnapshotId>,
) -> Result<InspectResult, ApplicationError> {
    let mut result = adapter.service.inspect(
        context,
        &InspectRequest {
            entity,
            snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
            detail: scriberig_app::DetailLevel::Detailed,
        },
    )?;
    if result.label.is_none() {
        result.label = adapter.explorer_labels.get(&entity).cloned();
    }
    Ok(result)
}

/// Lists project-aware Explorer rows.
pub(super) fn list_explorer<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    context: &ApplicationContext,
    entity: EntityId,
    snapshot: SnapshotId,
    continuation: Option<scriberig_query::ContinuationToken>,
) -> Result<crate::explorer::ExplorerPage, ApplicationError> {
    if continuation.is_none() && adapter.explorer_labels.contains_key(&entity) {
        let mut rows = adapter
            .explorer_labels
            .iter()
            .filter(|(candidate, _)| **candidate != entity)
            .map(|(candidate, label)| crate::explorer::ExplorerRow {
                selection: crate::selection::EntitySelection {
                    entity: *candidate,
                    revision: None,
                    snapshot: Some(snapshot),
                },
                label: label.clone(),
                kind: None,
                inspectable: true,
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| left.label.cmp(&right.label));
        return Ok(crate::explorer::ExplorerPage { rows, next: None });
    }
    let result = adapter.service.execute(
        context,
        &ApplicationRequest::List(scriberig_query::ListRequest {
            subject: scriberig_query::EntityInSnapshot { entity, snapshot },
            collection: scriberig_query::CollectionRef {
                owner: entity,
                collection_type: "snapshot.entities".to_owned(),
            },
            page: scriberig_query::PageRequest {
                continuation,
                ..Default::default()
            },
            detail: scriberig_query::DetailLevel::Compact,
        }),
    )?;
    let ApplicationResult::List(result) = result else {
        return Err(ApplicationError::Internal {
            message: "explorer listing returned a different result kind".to_owned(),
        });
    };
    let rows = result
        .page
        .items
        .into_iter()
        .filter_map(|entry| match entry.target {
            scriberig_query::TargetRef::Snapshot(target) => Some(crate::explorer::ExplorerRow {
                selection: crate::selection::EntitySelection {
                    entity: target.entity,
                    revision: None,
                    snapshot: Some(target.snapshot),
                },
                label: entry.label.unwrap_or_else(|| target.entity.to_string()),
                kind: entry.entity_type.map(|value| value.to_string()),
                inspectable: true,
            }),
            scriberig_query::TargetRef::Revision(_) => None,
        })
        .filter(|row| row.selection.entity != entity)
        .collect();
    Ok(crate::explorer::ExplorerPage {
        rows,
        next: result.page.next,
    })
}

/// Finds entities through the shared application query boundary.
pub(super) fn find_entities<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    context: &ApplicationContext,
    snapshot: SnapshotId,
    query: &str,
) -> Result<crate::view::find::FindModel, ApplicationError> {
    let result = adapter.service.execute(
        context,
        &ApplicationRequest::Find(scriberig_query::FindRequest {
            scope: scriberig_query::QueryScope::Snapshot(snapshot),
            query: query.to_owned(),
            strategy: scriberig_query::FindStrategy::ExactText,
            page: scriberig_query::PageRequest::default(),
            detail: scriberig_query::DetailLevel::Compact,
        }),
    )?;
    let ApplicationResult::Find(result) = result else {
        return Err(ApplicationError::Internal {
            message: "find returned a different result kind".to_owned(),
        });
    };
    Ok(crate::view::find::FindModel::from_result(query, &result))
}

/// Executes project-specific rendering, candidate creation, and application.
pub(super) fn execute_operation<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    context: &ApplicationContext,
    request: &ApplicationRequest,
) -> Result<Option<ApplicationResult>, ApplicationError> {
    if let ApplicationRequest::Apply(apply) = request
        && service_project_projection::parse_edit(&apply.change).is_none()
        && let Some(workflow) = adapter.project_workflow.as_ref()
        && let Some(result) =
            super::service_project_candidate_review::review_candidate(workflow, apply)?
    {
        return Ok(Some(result));
    }
    if let ApplicationRequest::Commit(commit) = request
        && let Some(workflow) = adapter.project_workflow.as_ref()
    {
        return super::service_project_candidate_review::apply_candidate(workflow, commit)
            .map(Some);
    }
    let Some(projection) = &adapter.project_projection else {
        return Ok(None);
    };
    if let ApplicationRequest::Render(render) = request
        && let Some(view) = render.view.strip_prefix("project-source:")
    {
        // The application resolves the style for this surface's purpose; this
        // module never chooses a style itself.
        let style = adapter
            .service
            .resolve_render_style(context, &render.style)?;
        return render_project_source(projection, view, &style).map(Some);
    }
    let _ = context;
    projected::execute_projected_change(adapter, projection, request)
}

/// Reads an operation record from the shared registry.
pub(super) fn operation_status<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    operation: &scriberig_core::OperationId,
) -> Result<scriberig_app::OperationRecord, ApplicationError> {
    adapter.service.operations().get(operation)
}

/// Cancels an operation through the shared registry.
pub(super) fn cancel_operation<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    operation: &scriberig_core::OperationId,
) -> Result<scriberig_app::OperationRecord, ApplicationError> {
    adapter.service.operations().cancel(operation)
}

/// Returns retained application lifecycle events.
pub(super) fn application_events<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
) -> Vec<scriberig_app::ApplicationEvent> {
    let cursor = adapter
        .event_cursor
        .lock()
        .map(|value| *value)
        .unwrap_or_default();
    let events = adapter.service.operations().events_with_sequences();
    let pending = events
        .iter()
        .filter(|(sequence, _)| cursor.is_none_or(|cursor| *sequence > cursor))
        .map(|(_, event)| event.clone())
        .collect::<Vec<_>>();
    if let Some(sequence) = events.iter().map(|(sequence, _)| *sequence).max()
        && let Ok(mut cursor) = adapter.event_cursor.lock()
    {
        *cursor = Some(sequence);
    }
    pending
}

/// Reads one session through the shared application boundary.
pub(super) fn read_session<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    context: &ApplicationContext,
    session: EntityId,
) -> Result<scriberig_session::Session, ApplicationError> {
    scriberig_app::SessionBackend::read_session(
        &adapter.service,
        context,
        &scriberig_app::SessionRequest { session },
    )
}

/// Submits an agent command through the shared session boundary.
pub(super) fn submit_agent_command<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    context: &ApplicationContext,
    session: EntityId,
    branch: Option<EntityId>,
    prompt: &str,
) -> Result<scriberig_core::OperationId, ApplicationError> {
    scriberig_app::SessionBackend::submit_agent_command(
        &adapter.service,
        context,
        &scriberig_app::AgentCommandRequest {
            session,
            branch,
            anchor: None,
            prompt: prompt.to_owned(),
            selection: None,
        },
    )
}

/// Builds capability descriptors exposed by the operation adapter.
pub(super) fn runtime_capabilities<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
) -> Result<Vec<RuntimeCapability>, ApplicationError> {
    let mut capabilities = vec![
        ("inspect_entity", "Inspect an exact semantic entity."),
        ("explorer_list", "List entities in an exact snapshot."),
        ("semantic_find", "Search entities in an exact snapshot."),
        ("history", "Read immutable snapshot history."),
        ("diff", "Compare two exact snapshots."),
        ("session_branch_tree", "Read session and branch state."),
        ("operation_status", "Observe operation lifecycle state."),
        ("operation_cancel", "Cancel a cancellable operation."),
        ("agent_start", "Start provider-neutral agent work."),
        ("agent_continue", "Continue work on a session branch."),
    ];
    if adapter.project_projection.is_some() {
        capabilities.push(("projection_render", "Render an imported project source."));
        capabilities.push((
            "projection_edit_candidate",
            "Review a source projection candidate.",
        ));
    }
    if adapter.project_workflow.is_some() {
        capabilities.push(("change_review", "Review and accept project candidates."));
    }
    capabilities
        .into_iter()
        .map(|(id, description)| {
            Ok(RuntimeCapability::new(
                scriberig_app::CapabilityId::new(id).map_err(|error| {
                    ApplicationError::Internal {
                        message: error.to_owned(),
                    }
                })?,
                description,
                true,
            ))
        })
        .collect()
}
