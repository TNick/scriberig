//! Implements candidate proposal and application HTTP operations.

#![allow(clippy::wildcard_imports)]

use super::*;
use scriberig_project::{ProjectSourceChange, ProjectSourceChangeKind};

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

/// Request body for one aggregate project source candidate.
#[derive(serde::Deserialize)]
pub(super) struct ProposeChangeInput {
    /// Project identity receiving the candidate.
    project: String,
    /// Exact active base snapshot.
    base_snapshot: String,
    /// Ordered source effect requests.
    effects: Vec<ProposeChangeEffectInput>,
}

/// Request body for one aggregate source effect.
#[derive(serde::Deserialize)]
struct ProposeChangeEffectInput {
    /// Project-relative path.
    path: String,
    /// Effect kind (`create`, `update`, or `delete`).
    kind: String,
    /// New bytes represented as UTF-8 text or a JSON byte array.
    content: Option<Value>,
}

/// Request body for candidate application.
#[derive(serde::Deserialize)]
struct ApplyInput {
    /// Stable candidate identity.
    candidate: String,
}

/// Request body for candidate detail or validation.
#[derive(serde::Deserialize)]
struct CandidateInput {
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
        CandidateValidation::Pending => "pending",
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

/// Creates one exact-base project-wide candidate through the shared project service.
pub(super) fn propose_change<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: ProposeChangeInput =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let project = input.project.parse().map_err(|_| invalid_project())?;
    let base_snapshot = input.base_snapshot.parse().map_err(|_| invalid_project())?;
    let changes = input
        .effects
        .into_iter()
        .map(|effect| {
            let kind = match effect.kind.as_str() {
                "create" => ProjectSourceChangeKind::Create,
                "update" => ProjectSourceChangeKind::Update,
                "delete" => ProjectSourceChangeKind::Delete,
                _ => return Err((400, json!({"code": "invalid_effect_kind"}))),
            };
            let content = match effect.content {
                Some(Value::String(text)) => Some(text.into_bytes()),
                Some(Value::Array(bytes)) => Some(
                    bytes
                        .into_iter()
                        .map(|byte| byte.as_u64().and_then(|number| u8::try_from(number).ok()))
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| (400, json!({"code": "invalid_effect_content"})))?,
                ),
                Some(Value::Null) | None => None,
                _ => return Err((400, json!({"code": "invalid_effect_content"}))),
            };
            Ok(ProjectSourceChange {
                path: effect.path,
                kind,
                content,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let candidate = ProjectService::new(Arc::clone(store))
        .propose_project_change(project, base_snapshot, changes)
        .map_err(|error| project_error(&error))?;
    Ok(json!({
        "candidate": candidate.id,
        "project": candidate.project.to_string(),
        "base": candidate.base_snapshot.to_string(),
        "validation": "pending",
        "effects": candidate.effects.iter().map(|effect| json!({
            "path": effect.path,
            "kind": match effect.kind {
                scriberig_project::ProjectSourceEffectKind::Create => "create",
                scriberig_project::ProjectSourceEffectKind::Update => "update",
                scriberig_project::ProjectSourceEffectKind::Delete => "delete",
            },
            "oldBytes": effect.old,
            "newBytes": effect.new,
        })).collect::<Vec<_>>(),
    }))
}

/// Loads a durable candidate and all of its path-level source effects.
///
/// - `state`: Shared application state.
/// - `body`: JSON candidate identity.
///
/// Candidate detail including ordered source effects.
///
/// Returns a structured request, storage, or not-found error.
pub(super) fn candidate<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: CandidateInput =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let candidate = ProjectService::new(Arc::clone(store))
        .candidate(&input.candidate)
        .map_err(|error| project_error(&error))?
        .ok_or_else(|| (404, json!({"code": "candidate_not_found"})))?;
    let mut result = candidate_json(&candidate);
    result["effectPaths"] = json!(store
        .project_effect_paths(&candidate.id)
        .map_err(storage_error)?
        .into_iter()
        .map(|receipt| json!({
            "path": receipt.path,
            "state": receipt.state,
            "expectedDigest": receipt.expected_digest.map(blake3::Hash::from_bytes).map(|hash| hash.to_hex().to_string()),
            "observedDigest": receipt.observed_digest.map(blake3::Hash::from_bytes).map(|hash| hash.to_hex().to_string()),
            "diagnostic": receipt.diagnostic,
        }))
        .collect::<Vec<_>>());
    Ok(result)
}

/// Validates one durable candidate in its isolated exact-base workspace.
///
/// - `state`: Shared application state.
/// - `body`: JSON candidate identity.
///
/// Candidate detail with durable validation state.
///
/// Returns a structured request, storage, stale-base, or not-found error.
pub(super) fn validate_candidate<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let store = state.project_store.as_ref().ok_or_else(unavailable)?;
    let input: CandidateInput =
        serde_json::from_str(body).map_err(|error| invalid_request(&error))?;
    let candidate = ProjectService::new(Arc::clone(store))
        .validate_candidate(&input.candidate)
        .map_err(|error| project_error(&error))?;
    Ok(candidate_json(&candidate))
}

/// Serializes a candidate using the shared effect representation.
///
/// - `candidate`: Durable project candidate.
///
/// Protocol-neutral JSON with exact base bytes and proposed bytes per path.
/// # Returns
/// The operation result.
fn candidate_json(candidate: &scriberig_project::ProjectCandidate) -> Value {
    let (validation, diagnostics) = match &candidate.validation {
        CandidateValidation::Pending => ("pending", Vec::new()),
        CandidateValidation::Passed => ("passed", Vec::new()),
        CandidateValidation::Failed { diagnostics } => ("failed", diagnostics.clone()),
    };
    json!({
        "candidate": candidate.id,
        "project": candidate.project.to_string(),
        "base": candidate.base_snapshot.to_string(),
        "validation": validation,
        "diagnostics": diagnostics,
        "effects": candidate.effects.iter().map(|effect| json!({
            "path": effect.path,
            "kind": match effect.kind {
                scriberig_project::ProjectSourceEffectKind::Create => "create",
                scriberig_project::ProjectSourceEffectKind::Update => "update",
                scriberig_project::ProjectSourceEffectKind::Delete => "delete",
            },
            "oldBytes": effect.old,
            "newBytes": effect.new,
        })).collect::<Vec<_>>(),
    })
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
            "validation_failed" | "validation_pending" => 422,
            _ => 500,
        };
        let mut response = json!({"code": error.code, "message": error.message});
        if let Ok(Some(effect)) = store.project_effect(&input.candidate) {
            response["effectState"] = json!(effect.state);
            response["effectDiagnostic"] = json!(effect.diagnostic);
        }
        if let Ok(effects) = store.project_effect_paths(&input.candidate) {
            response["effectPaths"] = json!(
                effects
                    .iter()
                    .map(|effect| json!({
                        "path": effect.path,
                        "state": effect.state,
                        "diagnostic": effect.diagnostic,
                    }))
                    .collect::<Vec<_>>()
            );
        }
        (status, response)
    })?;
    Ok(json!({
        "candidate": input.candidate,
        "base": report.base_snapshot.to_string(),
        "successor": report.successor_snapshot.to_string(),
        "path": report.path,
        "paths": report.paths,
        "materialization": report.effect_state,
        "history": true,
    }))
}
