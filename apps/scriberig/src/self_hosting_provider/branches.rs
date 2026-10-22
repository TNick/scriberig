//! Coordinates semantic discovery and implementation child branches.

#[path = "branches/queries.rs"]
mod queries;
use queries::{ARCHITECTURE_QUERIES, IMPLEMENTATION_QUERIES, PHP_CANDIDATE_PROFILES};
#[path = "branches/result_helpers.rs"]
mod result_helpers;
use result_helpers::{child_summary, is_child_result, is_successful_candidate_validation};

use super::{
    SemanticScope, ValidationMode, candidate, delegate, final_response, handle_with_prefix,
    malformed, parse_handles, semantic_entity, tool_call, tool_messages,
};
use rpi_ai::{AssistantMessage, Context, Model};
use scriberig_agent_runtime::host_configuration::HostConfigurationError;
use serde_json::json;

/// Advances the parent through capability discovery and two bounded child branches.
///
/// - `request`: Normalized parent branch request.
/// - `model`: Upstream model producing the next semantic tool call.
/// - `scope`: Exact project and snapshot references.
/// - `validation_failure`: Whether the scenario should exercise validation failure.
///
/// A trusted capability, semantic, child, or candidate inspection request.
///
/// Returns an error when a completed child omits its durable result reference.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn root_branch(
    request: &Context,
    model: &Model,
    scope: &SemanticScope,
    validation_failure: bool,
) -> Result<AssistantMessage, HostConfigurationError> {
    let tools = tool_messages(request);
    if tools.is_empty() {
        return Ok(tool_call(model, "runtime_capabilities", json!({})));
    }
    if !tools
        .iter()
        .any(|message| message.contains("semantic find"))
    {
        return Ok(tool_call(
            model,
            "semantic_find",
            json!({"project":scope.project,"snapshot":scope.snapshot,"query":"AGENTS.md","limit":10}),
        ));
    }
    if !tools
        .iter()
        .any(|message| message.contains("source_digest:"))
    {
        let entity = semantic_entity(
            tools
                .last()
                .ok_or_else(|| malformed("guidance find result missing"))?,
        )?;
        return Ok(tool_call(
            model,
            "semantic_render",
            json!({"project":scope.project,"snapshot":scope.snapshot,"entity":entity}),
        ));
    }
    if !tools.iter().any(|message| is_child_result(message)) {
        return Ok(delegate(
            model,
            "determine minimal extension points for PHP source ingestion",
            "ARCHITECTURE_DISCOVERY_TASK: Find and render the existing language adapter, PHP target contract, scanner, importer, workspace manifest, and project tests. Return a compact architecture map with exact semantic owner refs.",
            &["inspect.entity", "semantic_find", "semantic_render"],
            "compact architecture map with stable semantic refs",
        ));
    }
    let child_results = tools
        .iter()
        .filter(|message| is_child_result(message))
        .collect::<Vec<_>>();
    if child_results.is_empty() {
        return Err(malformed("architecture BranchResult missing"));
    }
    if child_results.len() == 1 {
        return implementation_child(model, child_results[0]);
    }
    if validation_failure
        && child_results
            .last()
            .is_some_and(|message| message.contains("validation failed"))
    {
        return Ok(final_response(
            model,
            &format!(
                "The PHP candidate failed isolated validation at exact base {}. It remains reviewable, and no Apply was performed.",
                scope.snapshot
            ),
        ));
    }
    if tools
        .iter()
        .any(|message| message.contains("project candidate detail"))
    {
        return Ok(final_response(
            model,
            &format!(
                "The validated PHP implementation candidate is ready for review at exact base {}. Review candidate before Apply.",
                scope.snapshot
            ),
        ));
    }
    let candidate = tools
        .iter()
        .rev()
        .find_map(|message| handle_with_prefix(message, "candidate:"))
        .ok_or_else(|| malformed("implementation BranchResult omitted CandidateId"))?;
    Ok(tool_call(
        model,
        "project_candidate",
        json!({"candidate":candidate}),
    ))
}

/// Delegates exact-base PHP implementation with candidate validation and dependency acquisition.
///
/// - `architecture`: Completed discovery child result and stable semantic references.
/// - `super`: Parent scenario module.
///
/// A bounded implementation child request.
///
/// - `model`: Upstream model producing the child delegation request.
///
/// Returns malformed-response when the architecture result omits its compact summary.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn implementation_child(
    model: &Model,
    architecture: &str,
) -> Result<AssistantMessage, HostConfigurationError> {
    let summary = child_summary(architecture)?;
    let references = parse_handles(architecture)?
        .into_iter()
        .filter(|handle| {
            handle.starts_with("entity:")
                || handle.starts_with("path:")
                || handle.starts_with("source_digest:")
        })
        .collect::<Vec<_>>();
    Ok(delegate(
        model,
        "implement PHP ingestion as one exact-base project candidate",
        &format!(
            "PHP_IMPLEMENTATION_TASK: Re-inspect exact Cargo, scanner, importer, and adapter targets at this project snapshot. Compact architecture BranchResult: {summary}; stable refs: {references:?}. Construct one candidate from the checked-in PHP source fixture, validate it, inspect its paths, and return CandidateId plus validation evidence."
        ),
        &[
            "inspect.entity",
            "semantic_find",
            "semantic_render",
            "project_propose",
            "project_candidate",
            "project_resolve_dependencies",
            "project_validate_candidate",
        ],
        "compact validated candidate result with durable refs",
    ))
}

/// Runs read-only architecture discovery and returns a compact result.
///
/// - `request`: Normalized architecture child request.
/// - `scope`: Exact project and snapshot references.
/// - `super`: Parent scenario module.
/// - `scope`: Exact project and snapshot inherited by the child.
///
/// A semantic search/render operation or the compact architecture result.
///
/// - `model`: Upstream model producing the architecture discovery response.
///
/// Returns an error when a source result lacks a semantic source handle.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn architecture_branch(
    request: &Context,
    model: &Model,
    scope: &SemanticScope,
) -> Result<AssistantMessage, HostConfigurationError> {
    match semantic_step(request, model, scope, ARCHITECTURE_QUERIES)? {
        Some(response) => Ok(response),
        None => Ok(final_response(
            model,
            "ARCHITECTURE_MAP: tree-sitter-python is the adapter precedent; scan.rs classifies extensions; support.rs dispatches persisted adapters; adapters.rs imports complete syntax and named symbols; Cargo.toml owns workspace membership; project tests own public import conformance.",
        )),
    }
}

/// Re-inspects implementation targets, proposes, validates, and inspects one candidate.
///
/// - `request`: Normalized implementation child request.
/// - `model`: Upstream model producing the semantic candidate request.
/// - `scope`: Exact project and snapshot references.
/// - `validation`: Validation mode for candidate materialization.
///
/// A regular project candidate tool call or a compact validated result.
///
/// Returns malformed-response when candidate handles are missing.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn implementation_branch(
    request: &Context,
    model: &Model,
    scope: &SemanticScope,
    validation: ValidationMode,
) -> Result<AssistantMessage, HostConfigurationError> {
    let tools = tool_messages(request);
    if let Some(response) = semantic_step(request, model, scope, IMPLEMENTATION_QUERIES)? {
        return Ok(response);
    }
    let proposals = tools
        .iter()
        .filter(|message| message.contains("project-wide candidate"))
        .collect::<Vec<_>>();
    if let Some(proposal) = proposals.last() {
        let candidate = handle_with_prefix(proposal, "candidate:")
            .ok_or_else(|| malformed("candidate proposal omitted identity"))?;
        let candidate_tools = tools
            .iter()
            .filter(|message| message.contains(&candidate))
            .collect::<Vec<_>>();
        if candidate_tools
            .iter()
            .any(|message| message.contains("validation:failed"))
        {
            if !candidate_tools
                .iter()
                .any(|message| message.contains("project candidate detail"))
            {
                return Ok(tool_call(
                    model,
                    "project_candidate",
                    json!({"candidate":candidate}),
                ));
            }
            if validation == ValidationMode::RepairFirst && proposals.len() == 1 {
                return Ok(candidate::project_proposal(model, scope, false));
            }
            return Ok(final_response(
                model,
                &format!(
                    "PHP_IMPLEMENTATION_RESULT: candidate {candidate} validation failed at exact base {}. No Apply was requested.",
                    scope.snapshot
                ),
            ));
        }
        if !candidate_tools
            .iter()
            .any(|message| message.contains("dependency_resolution:passed"))
        {
            return Ok(tool_call(
                model,
                "project_resolve_dependencies",
                json!({"candidate":candidate}),
            ));
        }
        let latest_profile_index = candidate_tools
            .iter()
            .rev()
            .find(|message| is_successful_candidate_validation(message, &candidate))
            .and_then(|message| {
                PHP_CANDIDATE_PROFILES.iter().position(|profile| {
                    message.contains(&format!("validation_id:{candidate}:{profile}"))
                })
            });
        let completed_profiles = latest_profile_index.map_or(0, |index| index + 1);
        if let Some(profile) = PHP_CANDIDATE_PROFILES.get(completed_profiles) {
            return Ok(tool_call(
                model,
                "project_validate_candidate",
                json!({"candidate":candidate,"profile":profile}),
            ));
        }
        if !candidate_tools
            .iter()
            .any(|message| message.contains("project candidate detail"))
        {
            return Ok(tool_call(
                model,
                "project_candidate",
                json!({"candidate":candidate}),
            ));
        }
        return Ok(final_response(
            model,
            &format!(
                "PHP_IMPLEMENTATION_RESULT: candidate {candidate} is validated at exact base {}. Review it before Apply.",
                scope.snapshot
            ),
        ));
    }
    Ok(candidate::project_proposal(
        model,
        scope,
        validation != ValidationMode::Normal,
    ))
}

/// Requests each semantic source target and renders its exact project projection.
///
/// - `request`: Current child model request.
/// - `scope`: Exact project and snapshot references.
/// - `queries`: Ordered semantic source path anchors.
/// - `super`: Parent scenario module.
/// - `scope`: Exact project and snapshot.
/// - `queries`: Ordered source path anchors.
/// - `model`: Upstream model producing the next search or render request.
///
/// Next semantic find/render response, or none after all targets are rendered.
///
/// Returns malformed-response when a find result lacks a source reference.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn semantic_step(
    request: &Context,
    model: &Model,
    scope: &SemanticScope,
    queries: &[&str],
) -> Result<Option<AssistantMessage>, HostConfigurationError> {
    let tools = tool_messages(request);
    let searched = tools
        .iter()
        .filter(|message| is_semantic_find(message))
        .count();
    let last_is_find = tools
        .last()
        .is_some_and(|message| is_semantic_find(message));
    if searched >= queries.len() && !last_is_find {
        return Ok(None);
    }
    if last_is_find {
        let result = tools.last().expect("last tool exists");
        match semantic_entity(result) {
            Ok(entity) => {
                return Ok(Some(tool_call(
                    model,
                    "semantic_render",
                    json!({"project":scope.project,"snapshot":scope.snapshot,"entity":entity}),
                )));
            }
            Err(_) if result.contains("matched 0 entities") => {
                if searched >= queries.len() {
                    return Ok(None);
                }
            }
            Err(error) => return Err(error),
        }
    }
    if searched >= queries.len() {
        return Ok(None);
    }
    Ok(Some(tool_call(
        model,
        "semantic_find",
        json!({
            "project":scope.project,
            "snapshot":scope.snapshot,
            "query":queries[searched],
            "limit":50
        }),
    )))
}

/// Checks whether one tool result is a semantic-find projection.
///
/// - `message`: Provider-normalized tool result text.
///
/// # Returns
/// `true` for the current JSON shape or the legacy debug shape.
fn is_semantic_find(message: &str) -> bool {
    message.contains("\"object_type\":\"semantic_find\"")
        || message.contains("object_type=Some(\"semantic_find\")")
}
