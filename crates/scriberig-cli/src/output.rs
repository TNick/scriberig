//! Owns terminal presentation for shared application inspection results.

use scriberig_app::InspectResult;

use crate::trust::{InspectDepth, OutputFormat, render_inspection_with_format};

/// Renders a protocol value as stable pretty JSON.
///
/// # Parameters
/// - `value`: Serializable public response value.
///
/// # Returns
/// Pretty-printed JSON text.
///
/// # Panics
/// Panics only if a public response type violates its serialization contract.
#[must_use]
pub fn render_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).expect("public response is serializable")
}

/// Renders an inspection result for a selected terminal representation.
///
/// # Parameters
/// - `result`: Shared application inspection result.
/// - `depth`: Requested trust detail.
/// - `format`: Human or structured output format.
///
/// # Returns
/// Terminal-ready inspection text.
#[must_use]
pub fn render_inspection_output(
    result: &InspectResult,
    depth: InspectDepth,
    format: OutputFormat,
) -> String {
    render_inspection_with_format(result, depth, format)
}

/// Renders one terminal agent result from the shared application result.
///
/// # Parameters
/// - `result`: Durable terminal agent result.
/// - `operation`: Stable operation identity.
/// - `format`: Human or structured output representation.
///
/// # Returns
/// Text intended for standard output.
#[must_use]
pub fn render_agent_result(
    result: &scriberig_app::AgentResult,
    operation: &str,
    format: crate::RunFormat,
) -> String {
    match format {
        crate::RunFormat::Human => result.final_output.clone(),
        crate::RunFormat::Json => serde_json::json!({
            "operation": operation,
            "session": result.session.to_string(),
            "root_branch": result.root_branch.to_string(),
            "final_output": result.final_output,
            "branch_result": result.branch_result.to_string(),
            "produced_changes": result
                .produced_changes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            "validation_results": result.validation_results,
            "unresolved_questions": result.unresolved_questions,
            "candidate_changes": result
                .candidate_changes
                .iter()
                .map(|change| {
                    serde_json::json!({
                        "migration": change.migration,
                        "entity": change.entity.map(|value| value.to_string()),
                        "candidate_snapshot": change
                            .candidate_snapshot
                            .map(|value| value.to_string()),
                        "validation": change.validation,
                    })
                })
                .collect::<Vec<_>>(),
        })
        .to_string(),
    }
}

/// Renders non-result diagnostics for the agent command's error stream.
///
/// # Parameters
/// - `result`: Durable terminal agent result.
/// - `operation`: Stable operation identity.
///
/// # Returns
/// Human-readable progress and identity diagnostics.
#[must_use]
pub fn render_agent_diagnostics(result: &scriberig_app::AgentResult, operation: &str) -> String {
    format!(
        "operation: {operation}\nsession: {}\nroot branch: {}",
        result.session, result.root_branch
    )
}

#[cfg(test)]
mod tests {
    use super::{render_agent_diagnostics, render_agent_result};
    use crate::RunFormat;
    use scriberig_app::AgentResult;

    use scriberig_core::EntityId;

    /// Creates a minimal stable agent result for output tests.
    ///
    /// # Returns
    /// A deterministic terminal result.
    ///
    /// # Errors
    /// This test helper never fails.
    fn result() -> AgentResult {
        AgentResult {
            session: EntityId::from_bytes([1; 16]),
            root_branch: EntityId::from_bytes([2; 16]),
            final_output: "answer".to_owned(),
            branch_result: EntityId::from_bytes([3; 16]),
            produced_changes: Vec::new(),
            validation_results: vec!["passed".to_owned()],
            unresolved_questions: Vec::new(),
            candidate_changes: Vec::new(),
        }
    }

    /// Verifies human output contains only the final semantic answer.
    #[test]
    fn human_agent_output_is_final_answer() {
        assert_eq!(
            render_agent_result(&result(), "op-1", RunFormat::Human),
            "answer"
        );
    }

    /// Verifies JSON output retains stable semantic handles.
    #[test]
    fn json_agent_output_contains_handles() {
        let output = render_agent_result(&result(), "op-1", RunFormat::Json);
        assert!(output.contains("\"operation\":\"op-1\""));
        assert!(output.contains("\"branch_result\""));
        assert!(output.contains("\"final_output\":\"answer\""));
    }

    /// Verifies diagnostics are separate from the final result payload.
    #[test]
    fn diagnostics_contain_identity_only() {
        let output = render_agent_diagnostics(&result(), "op-1");
        assert!(output.contains("operation: op-1"));
        assert!(output.contains("session:"));
        assert!(!output.contains("answer"));
    }
}
