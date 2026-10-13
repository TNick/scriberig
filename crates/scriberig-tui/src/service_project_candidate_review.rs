//! Resolves durable project candidates into shared application review results.

use scriberig_app::{
    ApplicationError, ApplicationResult, CommitRequest, OperationSummary, ValidationStatus,
};

use scriberig_project::ProjectService;

/// Loads an exact-base candidate for the shared change review operation.
///
/// # Parameters
/// - `workflow`: Shared durable project workflow.
/// - `request`: Candidate ID and exact base snapshot from the application request.
/// - `super`: Parent TUI service module.
///
/// # Returns
/// Candidate review data when the change handle is a durable project candidate.
///
/// # Errors
/// Returns storage, stale-base, or candidate retrieval errors.
pub(super) fn review_candidate(
    workflow: &ProjectService,
    request: &scriberig_app::ApplyRequest,
) -> Result<Option<ApplicationResult>, ApplicationError> {
    let Some(candidate) =
        workflow
            .candidate(&request.change)
            .map_err(|error| ApplicationError::OperationFailed {
                operation: error.code.to_owned(),
                message: error.message,
            })?
    else {
        return Ok(None);
    };
    if candidate.base_snapshot != request.base_snapshot {
        return Err(ApplicationError::StaleBaseSnapshot {
            requested: request.base_snapshot,
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
    let mut conflicts = vec![format!(
        "scope: entity={} snapshot={}",
        candidate.entity, candidate.base_snapshot
    )];
    conflicts.extend(
        candidate
            .effects
            .iter()
            .map(|effect| {
                format!(
                    "{} {}\nold:\n{}\nnew:\n{}",
                    effect_kind(effect.kind),
                    effect.path,
                    review_bytes(effect.old.as_deref()),
                    review_bytes(effect.new.as_deref())
                )
            })
            .collect::<Vec<_>>(),
    );
    Ok(Some(ApplicationResult::Apply(OperationSummary {
        handle: candidate.id,
        snapshot: Some(candidate.base_snapshot),
        revision: None,
        branch: None,
        diff: None,
        history: None,
        rendered: None,
        validation: Some(validation),
        validation_profile: candidate.validation_profile.clone(),
        validation_diagnostics: diagnostics,
        conflicts,
    })))
}

/// Applies one durable project candidate after explicit application confirmation.
///
/// # Parameters
/// - `workflow`: Shared project candidate service.
/// - `request`: Candidate ID and exact base snapshot from the commit operation.
/// - `super`: Parent TUI service module.
///
/// # Returns
/// Canonical successor and materialization receipt in the shared commit result.
///
/// # Errors
/// Returns missing, stale, validation, safety, or effect-recovery errors.
pub(super) fn apply_candidate(
    workflow: &ProjectService,
    request: &CommitRequest,
) -> Result<ApplicationResult, ApplicationError> {
    let candidate = workflow
        .candidate(&request.change)
        .map_err(|error| ApplicationError::OperationFailed {
            operation: error.code.to_owned(),
            message: error.message,
        })?
        .ok_or_else(|| ApplicationError::NotFound {
            subject: request.change.clone(),
        })?;
    if candidate.base_snapshot != request.base_snapshot {
        return Err(ApplicationError::StaleBaseSnapshot {
            requested: request.base_snapshot,
            current: candidate.base_snapshot,
        });
    }
    let report = workflow.apply_candidate(&candidate).map_err(|error| {
        ApplicationError::OperationFailed {
            operation: error.code.to_owned(),
            message: error.message,
        }
    })?;
    Ok(ApplicationResult::Commit(OperationSummary {
        handle: report.candidate,
        snapshot: Some(report.successor_snapshot),
        revision: None,
        branch: None,
        diff: None,
        history: None,
        rendered: None,
        validation: Some(ValidationStatus::Passed),
        validation_profile: candidate.validation_profile,
        validation_diagnostics: Vec::new(),
        conflicts: vec![format!("materialized {}", report.path)],
    }))
}

/// Returns a bounded, readable preview for candidate source bytes.
///
/// # Parameters
/// - `bytes`: Optional exact source bytes.
///
/// # Returns
/// UTF-8 preview or a stable marker for absent source data.
fn review_bytes(bytes: Option<&[u8]>) -> String {
    bytes.map_or_else(
        || "<absent>".to_owned(),
        |bytes| {
            let text = String::from_utf8_lossy(bytes);
            let mut preview = text.chars().take(500).collect::<String>();
            if text.chars().count() > 500 {
                preview.push_str("\n… truncated …");
            }
            preview
        },
    )
}

/// Returns the stable source effect label used in review content.
///
/// # Parameters
/// - `kind`: Candidate source effect kind.
///
/// # Returns
/// Lowercase effect label.
fn effect_kind(kind: scriberig_project::ProjectSourceEffectKind) -> &'static str {
    match kind {
        scriberig_project::ProjectSourceEffectKind::Create => "create",
        scriberig_project::ProjectSourceEffectKind::Update => "update",
        scriberig_project::ProjectSourceEffectKind::Delete => "delete",
    }
}
