//! Bounds durable branch results before they enter compiled provider context.

use super::CompactResultContext;
use scriberig_session::BranchResult;

const MAX_RESULT_SUMMARY_CHARS: usize = 2048;
const MAX_RESULT_HANDLES: usize = 32;
const SUMMARY_TRUNCATION_MARKER: &str = "\n[summary truncated]";

/// Projects a durable branch result to bounded summary and semantic handles.
///
/// # Parameters
/// - `result`: Full durable branch result.
///
/// # Returns
/// A compact context projection that leaves the durable result untouched.
///
/// # Errors
/// This projection is infallible.
pub fn compact_branch_result(result: &BranchResult) -> CompactResultContext {
    CompactResultContext {
        handle: format!("branch-result:{}", result.id),
        revision: result
            .candidate_context
            .map(|candidate| candidate.candidate_snapshot.to_string()),
        summary: result
            .summary
            .as_deref()
            .map(compact_summary)
            .unwrap_or_default(),
        semantic_handles: result
            .result_handles
            .iter()
            .take(MAX_RESULT_HANDLES)
            .cloned()
            .collect(),
    }
}

/// Truncates a branch summary while preserving a marker for omitted text.
///
/// # Parameters
/// - `summary`: Full persisted summary.
///
/// # Returns
/// The unchanged summary when within the limit, or a bounded marked prefix.
fn compact_summary(summary: &str) -> String {
    if summary.chars().count() <= MAX_RESULT_SUMMARY_CHARS {
        return summary.to_owned();
    }
    let prefix_limit = MAX_RESULT_SUMMARY_CHARS.saturating_sub(SUMMARY_TRUNCATION_MARKER.len());
    let mut compact = summary.chars().take(prefix_limit).collect::<String>();
    compact.push_str(SUMMARY_TRUNCATION_MARKER);
    compact
}
