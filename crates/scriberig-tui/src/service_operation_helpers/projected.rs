//! Implements projected candidate operations for the TUI adapter.

use super::*;

/// Executes legacy single-entity candidate requests against a project projection.
///
/// - `adapter`: Product adapter retaining candidate references.
/// - `projection`: Snapshot-bound project source data.
/// - `request`: Application request to execute.
///
/// A project result when the request matches the projected candidate workflow.
///
/// Returns validation, storage, stale-base, or missing-entity errors.
pub(super) fn execute_projected_change<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    projection: &super::service_project_projection::ProjectProjection,
    request: &ApplicationRequest,
) -> Result<Option<ApplicationResult>, ApplicationError> {
    if let ApplicationRequest::Apply(apply) = request
        && let Some((entity, replacement)) = service_project_projection::parse_edit(&apply.change)
    {
        return create_projected_candidate(adapter, projection, apply, entity, replacement)
            .map(Some);
    }
    if let ApplicationRequest::Apply(apply) = request
        && let Some(result) =
            super::service_project_candidate_review::review_candidate(&projection.workflow, apply)?
    {
        return Ok(Some(result));
    }
    if let ApplicationRequest::Commit(commit) = request {
        return apply_projected_candidate(adapter, projection, commit).map(Some);
    }
    Ok(None)
}

/// Creates and retains a projected single-entity candidate.
///
/// - `adapter`: Product adapter retaining candidate references.
/// - `projection`: Snapshot-bound source data.
/// - `apply`: Exact-base application request.
/// - `entity`: Source entity being edited.
/// - `replacement`: Replacement expression from the candidate request.
///
/// A summarized candidate result.
///
/// Returns source, validation, lock, or stale-base errors.
fn create_projected_candidate<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    projection: &super::service_project_projection::ProjectProjection,
    apply: &scriberig_app::ApplyRequest,
    entity: EntityId,
    replacement: &str,
) -> Result<ApplicationResult, ApplicationError> {
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
        scriberig_project::CandidateValidation::Pending => {
            (scriberig_app::ValidationStatus::Skipped, Vec::new())
        }
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
    Ok(ApplicationResult::Apply(scriberig_app::OperationSummary {
        handle: candidate.id,
        snapshot: Some(candidate.base_snapshot),
        revision: None,
        branch: None,
        diff: None,
        history: None,
        rendered: None,
        validation: Some(validation),
        validation_profile: None,
        validation_diagnostics: diagnostics,
        review_details: vec![format!(
            "{path}: add | old: left + right | new: {replacement}"
        )],
        conflicts: Vec::new(),
    }))
}

/// Applies a retained projected candidate through the project workflow.
///
/// - `adapter`: Product adapter retaining candidate references.
/// - `projection`: Snapshot-bound project workflow.
/// - `commit`: Exact-base commit request.
///
/// A commit result carrying the authoritative successor snapshot.
///
/// Returns stale-base, missing-candidate, or project-application errors.
fn apply_projected_candidate<B: ApplicationBackend>(
    adapter: &ApplicationOperationAdapter<B>,
    projection: &super::service_project_projection::ProjectProjection,
    commit: &scriberig_app::CommitRequest,
) -> Result<ApplicationResult, ApplicationError> {
    let cached = adapter
        .project_candidates
        .lock()
        .map_err(|_| ApplicationError::Internal {
            message: "project candidate lock poisoned".to_owned(),
        })?
        .get(&commit.change)
        .cloned();
    let candidate = match cached {
        Some(candidate) => candidate,
        None => projection
            .workflow
            .candidate(&commit.change)
            .map_err(|error| ApplicationError::OperationFailed {
                operation: error.code.to_owned(),
                message: error.message,
            })?
            .ok_or_else(|| ApplicationError::NotFound {
                subject: commit.change.clone(),
            })?,
    };
    if candidate.base_snapshot != commit.base_snapshot {
        return Err(ApplicationError::StaleBaseSnapshot {
            requested: commit.base_snapshot,
            current: candidate.base_snapshot,
        });
    }
    let report = projection
        .workflow
        .apply_candidate(&candidate)
        .map_err(|error| ApplicationError::OperationFailed {
            operation: error.code.to_owned(),
            message: error.message,
        })?;
    Ok(ApplicationResult::Commit(scriberig_app::OperationSummary {
        handle: report.candidate,
        snapshot: Some(report.successor_snapshot),
        revision: None,
        branch: None,
        diff: None,
        history: None,
        rendered: None,
        validation: Some(scriberig_app::ValidationStatus::Passed),
        validation_profile: None,
        validation_diagnostics: Vec::new(),
        review_details: vec![format!("materialized {}", report.path)],
        conflicts: Vec::new(),
    }))
}
