//! Resolves durable project candidates into shared application review results.

use scriberig_app::{
    ApplicationError, ApplicationResult, CommitRequest, OperationSummary, ValidationStatus,
};

use scriberig_project::ProjectService;

#[cfg(test)]
#[path = "service_project_candidate_review_tests.rs"]
mod tests;

/// Loads an exact-base candidate for the shared change review operation.
///
/// - `workflow`: Shared durable project workflow.
/// - `request`: Candidate ID and exact base snapshot from the application request.
/// - `super`: Parent TUI service module.
///
/// Candidate review data when the change handle is a durable project candidate.
///
/// Returns storage, stale-base, or candidate retrieval errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
    let mut review_details = vec![format!(
        "scope: entity={} snapshot={}",
        candidate.entity, candidate.base_snapshot
    )];
    if let Some(operation) = &candidate.semantic_operation {
        review_details.push(format!("semantic operation: {operation}"));
    }
    if let Some(target) = candidate.semantic_target {
        review_details.push(format!("semantic target: {target}"));
    }
    review_details.extend(semantic_change_details(&candidate.semantic_changes));
    review_details.extend(
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
        review_details,
        conflicts: Vec::new(),
    })))
}

/// Converts durable semantic before/after values into bounded TUI review details.
///
/// # Parameters
/// - `changes`: Typed semantic changes stored on the candidate.
///
/// # Returns
/// One compact user-facing review line for each semantic change.
fn semantic_change_details(changes: &[scriberig_project::SemanticCandidateChange]) -> Vec<String> {
    changes
        .iter()
        .map(|change| {
            format!(
                "semantic change: {} {} → {}",
                change.property,
                review_semantic_value(&change.before),
                review_semantic_value(&change.after)
            )
        })
        .collect()
}

/// Returns a bounded JSON preview for one semantic candidate value.
///
/// # Parameters
/// - `value`: Semantic value stored in the durable candidate.
///
/// # Returns
/// A compact JSON preview with an explicit truncation marker when needed.
fn review_semantic_value(value: &serde_json::Value) -> String {
    let display = match value {
        serde_json::Value::String(value) => format!("Text {}", serde_json::json!(value)),
        serde_json::Value::Object(object) if object.len() == 1 => {
            let (kind, payload) = object.iter().next().expect("one typed semantic value");
            if kind == "Formula" {
                payload
                    .get("expression")
                    .and_then(serde_json::Value::as_str)
                    .map_or_else(
                        || format!("Formula {payload}"),
                        |expression| format!("Formula {expression}"),
                    )
            } else {
                format!("{kind} {payload}")
            }
        }
        _ => value.to_string(),
    };
    let mut preview = display.chars().take(500).collect::<String>();
    if display.chars().count() > 500 {
        preview.push_str("… truncated …");
    }
    preview
}

/// Applies one durable project candidate after explicit application confirmation.
///
/// - `workflow`: Shared project candidate service.
/// - `request`: Candidate ID and exact base snapshot from the commit operation.
/// - `super`: Parent TUI service module.
///
/// Canonical successor and materialization receipt in the shared commit result.
///
/// Returns missing, stale, validation, safety, or effect-recovery errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
        let receipt = workflow
            .materialization_receipt(&request.change)
            .ok()
            .flatten();
        ApplicationError::OperationFailed {
            operation: error.code.to_owned(),
            message: append_materialization_receipt(&error.message, receipt),
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
        review_details: vec![format!("materialized {}", report.path)],
        conflicts: Vec::new(),
    }))
}

/// Adds bounded durable recovery state to a failed candidate apply message.
///
/// # Parameters
/// - `message`: Original safe materialization error.
/// - `receipt`: Aggregate and per-path state persisted by the project service.
///
/// # Returns
/// A terminal-safe message that states whether recovery is needed and which paths committed.
fn append_materialization_receipt(
    message: &str,
    receipt: Option<scriberig_project::ProjectMaterializationReceipt>,
) -> String {
    let Some(receipt) = receipt else {
        return message.to_owned();
    };
    let mut details = vec![format!("materialization state: {}", receipt.state)];
    if let Some(diagnostic) = receipt.diagnostic {
        details.push(crate::inspector::sanitize(&diagnostic));
    }
    for path in receipt.paths.iter().take(16) {
        let path_name: String = crate::inspector::sanitize(&path.path)
            .chars()
            .take(256)
            .collect();
        let diagnostic = path
            .diagnostic
            .as_deref()
            .map(crate::inspector::sanitize)
            .map(|value| value.chars().take(256).collect::<String>());
        details.push(match diagnostic {
            Some(diagnostic) => format!("{path_name}: {} ({diagnostic})", path.state),
            None => format!("{path_name}: {}", path.state),
        });
    }
    if receipt.paths.len() > 16 {
        details.push(format!(
            "{} more path receipts omitted",
            receipt.paths.len() - 16
        ));
    }
    format!("{message}; {}", details.join("; "))
}

/// Returns a bounded, readable preview for candidate source bytes.
///
/// - `bytes`: Optional exact source bytes.
///
/// UTF-8 preview or a stable marker for absent source data.
/// # Returns
/// The operation result.
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
/// - `kind`: Candidate source effect kind.
///
/// Lowercase effect label.
/// # Returns
/// The operation result.
fn effect_kind(kind: scriberig_project::ProjectSourceEffectKind) -> &'static str {
    match kind {
        scriberig_project::ProjectSourceEffectKind::Create => "create",
        scriberig_project::ProjectSourceEffectKind::Update => "update",
        scriberig_project::ProjectSourceEffectKind::Delete => "delete",
    }
}
