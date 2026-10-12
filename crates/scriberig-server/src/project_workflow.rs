//! Implements candidate proposal and application HTTP operations.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Request body for candidate creation.
#[derive(serde::Deserialize)]
pub(super) struct ProposeInput {
    /// Project identity receiving the candidate.
    pub project: String,
    /// Selected semantic entity.
    pub entity: String,
    /// Complete edited source projection.
    pub text: String,
}

/// Request body for candidate application.
#[derive(serde::Deserialize)]
struct ApplyInput {
    /// Stable candidate identity.
    candidate: String,
}

/// Creates and validates a source candidate without changing canonical state.
pub(super) fn propose<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ProposeInput =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let project = input.project.parse().map_err(|_| invalid_project())?;
    let entity = input.entity.parse().map_err(|_| invalid_project())?;
    let candidate = ProjectService::new(Arc::clone(store))
        .propose_candidate(project, entity, input.text)
        .map_err(|error| project_error(&error))?;
    let id = candidate.id.clone();
    let validation = match &candidate.validation {
        CandidateValidation::Passed => "passed",
        CandidateValidation::Failed { .. } => "failed",
    };
    let apply_enabled = matches!(candidate.validation, CandidateValidation::Passed);
    if let CandidateValidation::Failed { diagnostics } = &candidate.validation {
        return Err((
            400,
            json!({
                "code": "validation_failed",
                "message": diagnostics.join("\n"),
                "diagnostics": diagnostics,
            }),
        ));
    }
    Ok(json!({
        "candidate": id,
        "base": candidate.base_snapshot.to_string(),
        "path": candidate.path,
        "entity": candidate.entity.to_string(),
        "old": String::from_utf8_lossy(&candidate.old),
        "new": String::from_utf8_lossy(&candidate.new),
        "validation": validation,
        "applyEnabled": apply_enabled,
    }))
}

/// Applies a validated candidate with snapshot and filesystem compare-and-swap.
pub(super) fn apply<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ApplyInput = serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let service = ProjectService::new(Arc::clone(store));
    let candidate = service
        .candidate(&input.candidate)
        .map_err(|error| project_error(&error))?
        .ok_or_else(|| (404, json!({"code": "candidate_not_found"})))?;
    let report = service.apply_candidate(&candidate).map_err(|error| {
        let status = match error.code {
            "stale_candidate" | "filesystem_conflict" => 409,
            "validation_failed" => 422,
            _ => 500,
        };
        let mut response = json!({"code": error.code, "message": error.message});
        if let Ok(Some(effect)) = store.project_effect(&input.candidate) {
            response["effectState"] = json!(effect.state);
            response["effectDiagnostic"] = json!(effect.diagnostic);
        }
        (status, response)
    })?;
    Ok(json!({
        "candidate": input.candidate,
        "base": report.base_snapshot.to_string(),
        "successor": report.successor_snapshot.to_string(),
        "path": report.path,
        "materialization": report.effect_state,
        "history": true,
    }))
}
