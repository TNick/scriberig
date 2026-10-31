//! Reads stable branch and validation results from normalized tool records.

use scriberig_agent_runtime::host_configuration::HostConfigurationError;

use super::super::malformed;

/// Checks whether one tool message carries a completed child `BranchResult`.
///
/// - `message`: Provider-normalized tool result.
///
/// # Returns
/// `true` for the current JSON child-result shape or the legacy summary form.
pub(super) fn is_child_result(message: &str) -> bool {
    message.contains("\"branch_result\"") || message.contains("child_summary=")
}

/// Extracts the compact summary from one nested child result message.
///
/// - `message`: Provider-normalized child result.
///
/// # Returns
/// Child summary without its serialized stable handles.
///
/// # Errors
/// Returns an error when the child result omits its summary.
pub(super) fn child_summary(message: &str) -> Result<String, HostConfigurationError> {
    if let Some((_, content)) = message.split_once("child_summary=")
        && let Some((summary, _)) = content.split_once(" handles=[")
    {
        return Ok(summary.to_owned());
    }
    let start = message
        .find('{')
        .ok_or_else(|| malformed("child BranchResult summary missing"))?;
    let value: serde_json::Value = serde_json::from_str(&message[start..])
        .map_err(|_| malformed("child BranchResult summary missing"))?;
    value
        .get("summary")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
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
    let Some(value) = super::parse_tool_result(message) else {
        return false;
    };
    let summary = value.get("summary").and_then(serde_json::Value::as_str);
    let handles = value.get("handles").and_then(serde_json::Value::as_array);
    value.get("object_type").and_then(serde_json::Value::as_str) == Some("candidate_validation")
        && summary.is_some_and(|summary| {
            summary.contains(&format!("candidate {candidate} validation passed"))
        })
        && handles.is_some_and(|handles| {
            handles.iter().any(|handle| {
                handle.as_str().is_some_and(|handle| {
                    handle.starts_with(&format!("validation_id:{candidate}:"))
                })
            })
        })
}

#[cfg(test)]
mod tests {
    use super::is_successful_candidate_validation;

    /// Verifies JSON validation results advance the registered profile sequence.
    #[test]
    fn recognizes_json_candidate_validation_evidence() {
        let result = serde_json::json!({
            "object_type": "candidate_validation",
            "summary": "candidate cand_123 validation passed",
            "handles": ["validation_id:cand_123:php-domain-focused"]
        });
        let quoted =
            serde_json::to_string(&result.to_string()).expect("serialize quoted validation result");

        assert!(is_successful_candidate_validation(
            &result.to_string(),
            "cand_123"
        ));
        assert!(is_successful_candidate_validation(&quoted, "cand_123"));
        assert!(!is_successful_candidate_validation(
            &quoted,
            "other-candidate"
        ));
    }
}
