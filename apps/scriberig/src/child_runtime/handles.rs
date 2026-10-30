//! Retains bounded references produced by one agent branch.

/// Keeps recent stable handles from tool and child results.
///
/// - `result`: Completed child runtime result.
/// - `super`: Parent runtime module that owns result persistence.
///
/// Deduplicated handles suitable for a durable branch result.
/// # Returns
/// The operation result.
pub fn compact_result_handles(result: &scriberig_agent_runtime::AgentResult) -> Vec<String> {
    let mut primary = Vec::new();
    let mut details = Vec::new();
    for evidence in result.tool_evidence.iter().rev() {
        if evidence.failed {
            continue;
        }
        let Ok(output) = serde_json::from_slice::<serde_json::Value>(&evidence.output) else {
            continue;
        };
        let handles = output
            .get("handles")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .chain(
                evidence
                    .child_branch
                    .map(|branch| format!("branch:{branch}")),
            )
            .collect::<Vec<_>>();
        for handle in &handles {
            if !primary.contains(handle) && !details.contains(handle) {
                let destination = if is_primary_handle(handle) {
                    &mut primary
                } else {
                    &mut details
                };
                if destination.len() < 32 {
                    destination.push(handle.clone());
                }
            }
        }
        if primary.len() >= 32 {
            break;
        }
    }
    primary.extend(
        details
            .into_iter()
            .take(32usize.saturating_sub(primary.len())),
    );
    primary
}

/// Prioritizes typed references that callers need to continue durable work.
///
/// - `handle`: Stable result handle emitted by a tool or child branch.
///
/// `true` for core project, candidate, validation, or workspace identity handles.
/// # Returns
/// The operation result.
fn is_primary_handle(handle: &str) -> bool {
    [
        "candidate:",
        "validation:",
        "project:",
        "snapshot:",
        "manifest:",
        "root:",
        "entity:",
        "path:",
        "source_digest:",
    ]
    .iter()
    .any(|prefix| handle.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::compact_result_handles;
    use scriberig_agent_runtime::AgentResult;
    use scriberig_app::ToolExecutionEvidence;

    /// Keeps validation and candidate references when detail handles fill the bounded result.
    #[test]
    fn primary_candidate_handles_survive_detail_compaction() {
        let result = AgentResult {
            output: String::new(),
            model_requests: Vec::new(),
            tool_evidence: vec![ToolExecutionEvidence {
                ordinal: 0,
                name: "project_propose".to_owned(),
                input: Vec::new(),
                presentation_input: None,
                presentation_label: None,
                category: None,
                output: serde_json::json!({
                    "handles": (0..20)
                        .map(|index| format!("create:src/{index}.rs"))
                        .chain(["candidate:c1".to_owned(), "validation:passed".to_owned()])
                        .collect::<Vec<_>>(),
                })
                .to_string()
                .into_bytes(),
                failed: false,
                child_branch: None,
            }],
        };

        let handles = compact_result_handles(&result);
        assert!(handles.contains(&"candidate:c1".to_owned()));
        assert!(handles.contains(&"validation:passed".to_owned()));
        assert!(handles.len() <= 32);
    }
}
