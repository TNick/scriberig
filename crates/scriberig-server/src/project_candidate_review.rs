//! Serves bounded source-candidate review data without transmitting source bytes.

use std::sync::Arc;

use super::{
    ApplicationService, HttpState, Value, invalid_request, json, project_error, unavailable,
};
use scriberig_project::{CandidateValidation, ProjectCandidate};

/// Maximum source effects described by one review response.
const MAX_REVIEW_EFFECTS: usize = 32;
/// Maximum semantic values described by one review response.
const MAX_REVIEW_SEMANTIC_CHANGES: usize = 500;
/// Maximum validation diagnostics described by one review response.
const MAX_REVIEW_DIAGNOSTICS: usize = 20;
/// Maximum characters in one review path or diagnostic preview.
const MAX_REVIEW_TEXT_CHARS: usize = 512;

/// Describes the candidate identity requested by a review operation.
#[derive(serde::Deserialize)]
struct CandidateReviewInput {
    /// Stable durable candidate identity.
    candidate: String,
}

/// Loads one bounded review summary while keeping source bytes out of the response.
///
/// # Parameters
/// - `state`: Shared application state.
/// - `body`: JSON candidate identity.
///
/// # Returns
/// A bounded candidate review summary.
///
/// # Errors
/// Returns a structured request, storage, or not-found error.
pub(super) fn candidate_summary<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: CandidateReviewInput =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let candidate = state
        .service
        .project_service(Arc::clone(store))
        .candidate(&input.candidate)
        .map_err(|error| project_error(&error))?
        .ok_or_else(|| (404, json!({"code": "candidate_not_found"})))?;
    Ok(candidate_review_json(&candidate))
}

/// Validates one durable candidate and returns bounded review metadata.
///
/// # Parameters
/// - `state`: Shared application state.
/// - `body`: JSON candidate identity.
///
/// # Returns
/// Updated candidate validation and bounded effect metadata.
///
/// # Errors
/// Returns a structured request, storage, validation, stale-base, or not-found error.
pub(super) fn validate_candidate_summary<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: CandidateReviewInput =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let candidate = state
        .service
        .project_service(Arc::clone(store))
        .validate_candidate(&input.candidate)
        .map_err(|error| project_error(&error))?;
    Ok(candidate_review_json(&candidate))
}

/// Converts candidate validation and effects to a bounded response without source bytes.
///
/// # Parameters
/// - `candidate`: Durable project candidate being reviewed.
///
/// # Returns
/// JSON metadata with byte lengths, bounded paths, and bounded diagnostics.
fn candidate_review_json(candidate: &ProjectCandidate) -> Value {
    let (validation, diagnostics) = match &candidate.validation {
        CandidateValidation::Pending => ("pending", Vec::new()),
        CandidateValidation::Passed => ("passed", Vec::new()),
        CandidateValidation::Failed { diagnostics } => ("failed", diagnostics.clone()),
    };
    let effects = candidate
        .effects
        .iter()
        .take(MAX_REVIEW_EFFECTS)
        .map(|effect| {
            let (path, path_truncated) = bounded_review_text(&effect.path);
            json!({
                "path": path,
                "pathTruncated": path_truncated,
                "kind": match effect.kind {
                    scriberig_project::ProjectSourceEffectKind::Create => "create",
                    scriberig_project::ProjectSourceEffectKind::Update => "update",
                    scriberig_project::ProjectSourceEffectKind::Delete => "delete",
                },
                "oldByteLength": effect.old.as_ref().map_or(0, Vec::len),
                "newByteLength": effect.new.as_ref().map_or(0, Vec::len),
            })
        })
        .collect::<Vec<_>>();
    let diagnostics = diagnostics
        .iter()
        .take(MAX_REVIEW_DIAGNOSTICS)
        .map(|message| bounded_review_text(message).0)
        .collect::<Vec<_>>();
    let semantic_changes = candidate
        .semantic_changes
        .iter()
        .take(MAX_REVIEW_SEMANTIC_CHANGES)
        .map(|change| {
            let before = serde_json::to_string(&change.before).unwrap_or_default();
            let after = serde_json::to_string(&change.after).unwrap_or_default();
            let (before, before_truncated) = bounded_review_text(&before);
            let (after, after_truncated) = bounded_review_text(&after);
            json!({
                "target": change.target.to_string(),
                "property": bounded_review_text(&change.property).0,
                "before": before,
                "beforeTruncated": before_truncated,
                "after": after,
                "afterTruncated": after_truncated,
            })
        })
        .collect::<Vec<_>>();
    let (path, path_truncated) = bounded_review_text(&candidate.path);
    json!({
        "candidate": candidate.id,
        "project": candidate.project.to_string(),
        "base": candidate.base_snapshot.to_string(),
        "entity": candidate.entity.to_string(),
        "semanticOperation": candidate.semantic_operation.as_deref(),
        "semanticTarget": candidate.semantic_target.map(|target| target.to_string()),
        "semanticChanges": semantic_changes,
        "semanticChangesOmitted": candidate.semantic_changes.len().saturating_sub(MAX_REVIEW_SEMANTIC_CHANGES),
        "path": path,
        "pathTruncated": path_truncated,
        "validation": validation,
        "diagnostics": diagnostics,
        "effects": effects,
        "effectCount": candidate.effects.len(),
        "effectsOmitted": candidate.effects.len().saturating_sub(MAX_REVIEW_EFFECTS),
        "sourceBytesIncluded": false,
    })
}

/// Truncates one review string without splitting a UTF-8 code point.
///
/// # Parameters
/// - `value`: Source path or diagnostic text.
///
/// # Returns
/// A bounded text preview and whether characters were omitted.
fn bounded_review_text(value: &str) -> (String, bool) {
    let mut characters = value.chars();
    let preview = characters
        .by_ref()
        .take(MAX_REVIEW_TEXT_CHARS)
        .collect::<String>();
    (preview, characters.next().is_some())
}

#[cfg(test)]
mod tests {
    use super::candidate_review_json;
    use scriberig_core::{EntityId, ProjectId, SnapshotId};
    use scriberig_project::{
        CandidateValidation, ProjectCandidate, ProjectSourceEffect, ProjectSourceEffectKind,
    };

    /// Preserves durable semantic operation and target in the public review summary.
    #[test]
    fn candidate_summary_exposes_semantic_operation_and_target_without_source_bytes() {
        let target = EntityId::new();
        let candidate = ProjectCandidate {
            id: "candidate-1".to_owned(),
            project: ProjectId::fresh(),
            base_snapshot: SnapshotId::new([4; 32]),
            entity: EntityId::new(),
            semantic_operation: Some("spreadsheet.set_cell".to_owned()),
            semantic_target: Some(target),
            semantic_changes: vec![scriberig_project::SemanticCandidateChange {
                target,
                property: "cell.value".to_owned(),
                before: serde_json::json!({"Number": "10"}),
                after: serde_json::json!({"Number": "12"}),
            }],
            path: "budget.xlsx".to_owned(),
            old: b"old private source".to_vec(),
            new: b"new private source".to_vec(),
            effects: vec![ProjectSourceEffect {
                path: "budget.xlsx".to_owned(),
                kind: ProjectSourceEffectKind::Update,
                old: Some(b"old private source".to_vec()),
                observed: None,
                new: Some(b"new private source".to_vec()),
            }],
            validation: CandidateValidation::Pending,
            validation_profile: None,
            validation_digest: None,
        };

        let summary = candidate_review_json(&candidate);

        assert_eq!(summary["semanticOperation"], "spreadsheet.set_cell");
        assert_eq!(summary["semanticTarget"], target.to_string());
        assert_eq!(summary["semanticChanges"][0]["property"], "cell.value");
        assert_eq!(
            summary["semanticChanges"][0]["before"],
            r#"{"Number":"10"}"#
        );
        assert_eq!(summary["semanticChanges"][0]["after"], r#"{"Number":"12"}"#);
        assert_eq!(summary["semanticChangesOmitted"], 0);
        assert_eq!(summary["sourceBytesIncluded"], false);
        assert!(summary.get("old").is_none());
        assert!(summary.get("new").is_none());
    }

    /// Includes every semantic item in one maximum-size spreadsheet range edit.
    #[test]
    fn candidate_summary_preserves_all_bounded_range_changes() {
        let target = EntityId::new();
        let mut candidate = ProjectCandidate {
            id: "candidate-range".to_owned(),
            project: ProjectId::fresh(),
            base_snapshot: SnapshotId::new([4; 32]),
            entity: EntityId::new(),
            semantic_operation: Some("spreadsheet.set_range".to_owned()),
            semantic_target: Some(target),
            semantic_changes: Vec::new(),
            path: "budget.xlsx".to_owned(),
            old: Vec::new(),
            new: Vec::new(),
            effects: Vec::new(),
            validation: CandidateValidation::Pending,
            validation_profile: None,
            validation_digest: None,
        };
        candidate.semantic_changes = (0..500)
            .map(|index| scriberig_project::SemanticCandidateChange {
                target: EntityId::new(),
                property: format!("cell[{index}].value"),
                before: serde_json::json!({"Number": index.to_string()}),
                after: serde_json::json!({"Number": (index + 1).to_string()}),
            })
            .collect();

        let summary = candidate_review_json(&candidate);

        assert_eq!(summary["semanticChanges"].as_array().unwrap().len(), 500);
        assert_eq!(summary["semanticChangesOmitted"], 0);
        assert_eq!(
            summary["semanticChanges"][499]["property"],
            "cell[499].value"
        );
    }
}
