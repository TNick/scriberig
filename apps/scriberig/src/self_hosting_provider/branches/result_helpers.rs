//! Reads stable branch and validation results from normalized tool records.

use scriberig_agent_runtime::host_configuration::HostConfigurationError;

use super::super::malformed;

/// Extracts the compact summary from one nested child result message.
///
/// - `message`: Provider-normalized child result.
///
/// # Returns
/// Child summary without its serialized stable handles.
///
/// # Errors
/// Returns an error when the child result omits its summary.
pub(super) fn child_summary(message: &str) -> Result<&str, HostConfigurationError> {
    message
        .split_once("child_summary=")
        .and_then(|(_, content)| content.split_once(" handles=[").map(|(summary, _)| summary))
        .ok_or_else(|| malformed("child BranchResult summary missing"))
}

/// Checks for a successful validation result for the exact candidate.
///
/// - `message`: Serialized runtime tool result.
/// - `candidate`: Exact durable candidate identity.
///
/// # Returns
/// `true` when output contains a passed validation ID for this candidate.
pub(super) fn is_successful_candidate_validation(message: &str, candidate: &str) -> bool {
    message.contains("object_type=Some(\"candidate_validation\")")
        && message.contains(&format!(
            "summary=Some(\"candidate {candidate} validation passed\""
        ))
        && message.contains(&format!("validation_id:{candidate}:"))
}
