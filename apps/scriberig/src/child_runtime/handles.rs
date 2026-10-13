//! Retains bounded references produced by one agent branch.

/// Keeps recent stable handles from tool and child results.
///
/// # Parameters
/// - `result`: Completed child runtime result.
/// - `super`: Parent runtime module that owns result persistence.
///
/// # Returns
/// Deduplicated handles suitable for a durable branch result.
pub fn compact_result_handles(result: &scriberig_agent_runtime::AgentResult) -> Vec<String> {
    let mut primary = Vec::new();
    let mut details = Vec::new();
    for message in result.messages.iter().rev() {
        let recent = match message {
            scriberig_agent_runtime::AgentMessage::Tool(tool) => &tool.handles,
            scriberig_agent_runtime::AgentMessage::Child(child) => &child.handles,
            scriberig_agent_runtime::AgentMessage::User(_) => continue,
        };
        for handle in recent {
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
/// # Parameters
/// - `handle`: Stable result handle emitted by a tool or child branch.
///
/// # Returns
/// `true` for core project, candidate, validation, or workspace identity handles.
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
    use scriberig_agent_core::ToolResult;
    use scriberig_agent_runtime::{AgentMessage, AgentResult};
    use scriberig_core::EntityId;

    /// Keeps validation and candidate references when detail handles fill the bounded result.
    #[test]
    fn primary_candidate_handles_survive_detail_compaction() {
        let result = AgentResult {
            output: String::new(),
            messages: vec![AgentMessage::Tool(ToolResult {
                entity: EntityId::from_bytes([0; 16]),
                revision: None,
                snapshot: None,
                object_type: None,
                summary: None,
                handles: (0..20)
                    .map(|index| format!("create:src/{index}.rs"))
                    .chain(["candidate:c1".to_owned(), "validation:passed".to_owned()])
                    .collect(),
            })],
        };

        let handles = compact_result_handles(&result);
        assert!(handles.contains(&"candidate:c1".to_owned()));
        assert!(handles.contains(&"validation:passed".to_owned()));
        assert!(handles.len() <= 32);
    }
}
