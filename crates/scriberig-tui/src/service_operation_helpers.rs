//! Implements project-aware operation adapter helpers.

#![allow(clippy::wildcard_imports)]

use super::*;

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
    let Some(projection) = &adapter.project_projection else {
        return Ok(None);
    };
    if let ApplicationRequest::Render(render) = request
        && let Some(view) = render.view.strip_prefix("project-source:")
    {
        return render_project_source(projection, view).map(Some);
    }
    if let ApplicationRequest::Apply(apply) = request
        && let Some((entity, replacement)) = service_project_projection::parse_edit(&apply.change)
    {
        let Some((path, bytes)) = projection.sources.get(&entity) else {
            return Err(ApplicationError::NotFound {
                subject: entity.to_string(),
            });
        };
        let edited = String::from_utf8_lossy(bytes).replace("left + right", replacement);
        let candidate = projection
            .workflow
            .propose_candidate(projection.project, entity, edited)
            .map_err(|error| ApplicationError::ValidationFailure {
                message: error.to_string(),
            })?;
        if candidate.base_snapshot != apply.base_snapshot {
            return Err(ApplicationError::StaleBaseSnapshot {
                requested: apply.base_snapshot,
                current: candidate.base_snapshot,
            });
        }
        let (validation, diagnostics) = match &candidate.validation {
            scriberig_project::CandidateValidation::Passed => {
                (scriberig_app::ValidationStatus::Passed, Vec::new())
            }
            scriberig_project::CandidateValidation::Failed { diagnostics } => {
                (scriberig_app::ValidationStatus::Failed, diagnostics.clone())
            }
        };
        adapter
            .project_candidates
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "project candidate lock poisoned".to_owned(),
            })?
            .insert(candidate.id.clone(), candidate.clone());
        return Ok(Some(ApplicationResult::Apply(
            scriberig_app::OperationSummary {
                handle: candidate.id,
                snapshot: Some(candidate.base_snapshot),
                revision: None,
                branch: None,
                diff: None,
                history: None,
                rendered: None,
                validation: Some(validation),
                validation_diagnostics: diagnostics,
                conflicts: vec![format!(
                    "{path}: add | old: left + right | new: {replacement}"
                )],
            },
        )));
    }
    if let ApplicationRequest::Commit(commit) = request {
        let candidate = adapter
            .project_candidates
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "project candidate lock poisoned".to_owned(),
            })?
            .get(&commit.change)
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                subject: commit.change.clone(),
            })?;
        let report = projection
            .workflow
            .apply_candidate(&candidate)
            .map_err(|error| ApplicationError::OperationFailed {
                operation: error.code.to_owned(),
                message: error.message,
            })?;
        return Ok(Some(ApplicationResult::Commit(
            scriberig_app::OperationSummary {
                handle: report.candidate,
                snapshot: Some(report.successor_snapshot),
                revision: None,
                branch: None,
                diff: None,
                history: None,
                rendered: None,
                validation: Some(scriberig_app::ValidationStatus::Passed),
                validation_diagnostics: Vec::new(),
                conflicts: vec![format!("materialized {}", report.path)],
            },
        )));
    }
    let _ = context;
    Ok(None)
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
    let _ = adapter;
    adapter.service.operations().events()
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
