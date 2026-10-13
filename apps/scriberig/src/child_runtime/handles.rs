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
    let mut handles = Vec::new();
    for message in result.messages.iter().rev() {
        let recent = match message {
            scriberig_agent_runtime::AgentMessage::Tool(tool) => &tool.handles,
            scriberig_agent_runtime::AgentMessage::Child(child) => &child.handles,
            scriberig_agent_runtime::AgentMessage::User(_) => continue,
        };
        for handle in recent.iter().take(16) {
            if !handles.contains(handle) {
                handles.push(handle.clone());
            }
        }
        if handles.len() >= 32 {
            break;
        }
    }
    handles.truncate(32);
    handles
}
