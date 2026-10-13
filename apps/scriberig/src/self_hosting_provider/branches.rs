//! Coordinates semantic discovery and implementation child branches.

/// Read-only source references inspected by the architecture branch.
const ARCHITECTURE_QUERIES: &[&str] = &[
    "scriberig-domain-python",
    "design/level-2/028",
    "crates/scriberig-project/src/scan.rs",
    "crates/scriberig-project/src/support.rs",
    "crates/scriberig-project/src/adapters.rs",
    "Cargo.toml",
    "crates/scriberig-project/tests",
];

/// Exact target sources re-inspected by the implementation branch.
const IMPLEMENTATION_QUERIES: &[&str] = &[
    "Cargo.toml",
    "crates/scriberig-project/Cargo.toml",
    "crates/scriberig-project/src/scan.rs",
    "crates/scriberig-project/src/adapters.rs",
    "crates/scriberig-project/src/support.rs",
];

use super::{
    SemanticScope, candidate, delegate, final_response, handle_with_prefix, malformed,
    parse_handles, semantic_entity, tool_call, tool_messages,
};
use scriberig_agent_runtime::provider::{ModelRequest, ModelResponse, ProviderError};
use serde_json::json;

/// Advances the parent through capability discovery and two bounded child branches.
///
/// # Parameters
/// - `request`: Normalized parent branch request.
/// - `scope`: Exact project and snapshot references.
/// - `super`: Parent scenario module.
///
/// # Returns
/// A trusted capability, semantic, child, or candidate inspection request.
///
/// # Errors
/// Returns an error when a completed child omits its durable result reference.
pub(super) fn root_branch(
    request: &ModelRequest,
    scope: &SemanticScope,
    validation_failure: bool,
) -> Result<ModelResponse, ProviderError> {
    let tools = tool_messages(request);
    if tools.is_empty() {
        return Ok(tool_call("runtime_capabilities", json!({})));
    }
    if !tools
        .iter()
        .any(|message| message.contains("semantic find"))
    {
        return Ok(tool_call(
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
            "semantic_render",
            json!({"project":scope.project,"snapshot":scope.snapshot,"entity":entity}),
        ));
    }
    if !tools
        .iter()
        .any(|message| message.contains("child_summary="))
    {
        return Ok(delegate(
            "determine minimal extension points for PHP source ingestion",
            "ARCHITECTURE_DISCOVERY_TASK: Find and render the existing language adapter, PHP target contract, scanner, importer, workspace manifest, and project tests. Return a compact architecture map with exact semantic owner refs.",
            &["inspect.entity", "semantic_find", "semantic_render"],
            "compact architecture map with stable semantic refs",
        ));
    }
    let child_results = tools
        .iter()
        .filter(|message| message.contains("child_summary="))
        .collect::<Vec<_>>();
    if child_results.is_empty() {
        return Err(malformed("architecture BranchResult missing"));
    }
    if child_results.len() == 1 {
        let architecture = child_results[0];
        let summary = child_summary(architecture)?;
        let references = parse_handles(architecture)?
            .into_iter()
            .filter(|handle| {
                handle.starts_with("entity:")
                    || handle.starts_with("path:")
                    || handle.starts_with("source_digest:")
            })
            .collect::<Vec<_>>();
        return Ok(delegate(
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
                "project_validate_candidate",
            ],
            "compact validated candidate result with durable refs",
        ));
    }
    if validation_failure
        && child_results
            .last()
            .is_some_and(|message| message.contains("validation failed"))
    {
        return Ok(final_response(&format!(
            "The PHP candidate failed isolated validation at exact base {}. It remains reviewable, and no Apply was performed.",
            scope.snapshot
        )));
    }
    if tools
        .iter()
        .any(|message| message.contains("project candidate detail"))
    {
        return Ok(final_response(&format!(
            "The validated PHP implementation candidate is ready for review at exact base {}. Review candidate before Apply.",
            scope.snapshot
        )));
    }
    let candidate = tools
        .iter()
        .rev()
        .find_map(|message| handle_with_prefix(message, "candidate:"))
        .ok_or_else(|| malformed("implementation BranchResult omitted CandidateId"))?;
    Ok(tool_call(
        "project_candidate",
        json!({"candidate":candidate}),
    ))
}

/// Extracts the compact summary from one nested child result message.
///
/// # Parameters
/// - `message`: Provider-normalized child result.
/// - `super`: Parent branch coordinator.
///
/// # Returns
/// Child summary without its serialized stable handles.
///
/// # Errors
/// Returns malformed-response when the child result omits its summary.
fn child_summary(message: &str) -> Result<&str, ProviderError> {
    message
        .split_once("child_summary=")
        .and_then(|(_, content)| content.split_once(" handles=[").map(|(summary, _)| summary))
        .ok_or_else(|| malformed("child BranchResult summary missing"))
}

/// Runs read-only architecture discovery and returns a compact result.
///
/// # Parameters
/// - `request`: Normalized architecture child request.
/// - `scope`: Exact project and snapshot references.
/// - `super`: Parent scenario module.
/// - `scope`: Exact project and snapshot inherited by the child.
///
/// # Returns
/// A semantic search/render operation or the compact architecture result.
///
/// # Errors
/// Returns an error when a source result lacks a semantic source handle.
pub(super) fn architecture_branch(
    request: &ModelRequest,
    scope: &SemanticScope,
) -> Result<ModelResponse, ProviderError> {
    match semantic_step(request, scope, ARCHITECTURE_QUERIES)? {
        Some(response) => Ok(response),
        None => Ok(final_response(
            "ARCHITECTURE_MAP: tree-sitter-python is the adapter precedent; scan.rs classifies extensions; support.rs dispatches persisted adapters; adapters.rs imports complete syntax and named symbols; Cargo.toml owns workspace membership; project tests own public import conformance.",
        )),
    }
}

/// Re-inspects implementation targets, proposes, validates, and inspects one candidate.
///
/// # Parameters
/// - `request`: Normalized implementation child request.
/// - `scope`: Exact project and snapshot references.
/// - `super`: Parent scenario module.
/// - `scope`: Exact project and snapshot inherited by the child.
///
/// # Returns
/// A regular project candidate tool call or a compact validated result.
///
/// # Errors
/// Returns malformed-response when candidate handles are missing.
pub(super) fn implementation_branch(
    request: &ModelRequest,
    scope: &SemanticScope,
    validation_failure: bool,
) -> Result<ModelResponse, ProviderError> {
    let tools = tool_messages(request);
    if let Some(response) = semantic_step(request, scope, IMPLEMENTATION_QUERIES)? {
        return Ok(response);
    }
    if let Some(proposal) = tools
        .iter()
        .find(|message| message.contains("project-wide candidate"))
    {
        let candidate = handle_with_prefix(proposal, "candidate:")
            .ok_or_else(|| malformed("candidate proposal omitted identity"))?;
        if validation_failure
            && tools
                .iter()
                .any(|message| message.contains("validation failed"))
        {
            if !tools
                .iter()
                .any(|message| message.contains("project candidate detail"))
            {
                return Ok(tool_call(
                    "project_candidate",
                    json!({"candidate":candidate}),
                ));
            }
            return Ok(final_response(&format!(
                "PHP_IMPLEMENTATION_RESULT: candidate {candidate} validation failed at exact base {}. No Apply was requested.",
                scope.snapshot
            )));
        }
        if !tools
            .iter()
            .any(|message| message.contains("validation passed"))
        {
            return Ok(tool_call(
                "project_validate_candidate",
                json!({"candidate":candidate}),
            ));
        }
        if !tools
            .iter()
            .any(|message| message.contains("project candidate detail"))
        {
            return Ok(tool_call(
                "project_candidate",
                json!({"candidate":candidate}),
            ));
        }
        return Ok(final_response(&format!(
            "PHP_IMPLEMENTATION_RESULT: candidate {candidate} is validated at exact base {}. Review it before Apply.",
            scope.snapshot
        )));
    }
    Ok(candidate::project_proposal(scope, validation_failure))
}

/// Requests each semantic source target and renders its exact project projection.
///
/// # Parameters
/// - `request`: Current child model request.
/// - `scope`: Exact project and snapshot references.
/// - `queries`: Ordered semantic source path anchors.
/// - `super`: Parent scenario module.
/// - `scope`: Exact project and snapshot.
/// - `queries`: Ordered source path anchors.
///
/// # Returns
/// Next semantic find/render response, or none after all targets are rendered.
///
/// # Errors
/// Returns malformed-response when a find result lacks a source reference.
fn semantic_step(
    request: &ModelRequest,
    scope: &SemanticScope,
    queries: &[&str],
) -> Result<Option<ModelResponse>, ProviderError> {
    let tools = tool_messages(request);
    let rendered = tools
        .iter()
        .filter(|message| message.contains("source_digest:"))
        .count();
    if rendered >= queries.len() {
        return Ok(None);
    }
    if tools
        .last()
        .is_some_and(|message| message.contains("summary=Some(\"semantic find"))
    {
        let entity = semantic_entity(tools.last().expect("last tool exists"))?;
        return Ok(Some(tool_call(
            "semantic_render",
            json!({"project":scope.project,"snapshot":scope.snapshot,"entity":entity}),
        )));
    }
    Ok(Some(tool_call(
        "semantic_find",
        json!({
            "project":scope.project,
            "snapshot":scope.snapshot,
            "query":queries[rendered],
            "limit":50
        }),
    )))
}
