//! Defines bounded durable branch result summaries and semantic handles.

use crate::Session;
use scriberig_core::EntityId;

const MAX_SUMMARY_CHARS: usize = 2_048;
const MAX_RESULT_HANDLES: usize = 32;
const MAX_RESULT_HANDLE_CHARS: usize = 256;

/// Structured result produced by one completed or failed semantic branch.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct BranchResult {
    /// Stable result identity.
    pub id: EntityId,
    /// Branch that produced the result.
    pub branch: EntityId,
    /// Terminal or current branch status.
    pub status: crate::branch::BranchStatus,
    /// Compact human-readable result summary.
    pub summary: Option<String>,
    /// Safe result category.
    pub result_kind: Option<String>,
    /// Exact project/candidate snapshots associated with this result.
    #[serde(default)]
    pub candidate_context: Option<crate::types::BranchCandidateContext>,
    /// Semantic entity identities referenced by this result.
    pub references: Vec<EntityId>,
    /// Stable non-entity handles produced by the branch.
    #[serde(default)]
    pub result_handles: Vec<String>,
    /// Semantic entities touched by the branch.
    pub touched: Vec<EntityId>,
    /// Durable semantic changes produced by the branch.
    pub produced_changes: Vec<EntityId>,
    /// Validation records produced by the branch.
    pub validations: Vec<EntityId>,
    /// Questions left unresolved by the branch.
    pub unresolved_questions: Vec<EntityId>,
}

impl BranchResult {
    /// Bounds a durable summary and its opaque semantic handle collection.
    ///
    /// It preserves the first 32 safe handles in their original order. A
    /// bounded marker in `summary` makes summary or handle omissions visible.
    ///
    /// # Parameters
    /// - `self`: Branch result to compact before persistence.
    ///
    /// # Returns
    /// `true` when any persisted field was shortened or removed.
    pub fn compact_for_storage(&mut self) -> bool {
        let original_handles = self.result_handles.len();
        self.result_handles.retain(|handle| {
            !handle.is_empty() && handle.chars().count() <= MAX_RESULT_HANDLE_CHARS
        });
        self.result_handles.truncate(MAX_RESULT_HANDLES);
        let omitted_handles = original_handles.saturating_sub(self.result_handles.len());

        let original_summary = self.summary.take();
        let summary = original_summary.as_deref().unwrap_or_default();
        let handle_marker = format!("\n[{omitted_handles} branch result handles omitted]");
        let summary_length = summary.chars().count();
        let summary_was_truncated = summary_length > MAX_SUMMARY_CHARS
            || (omitted_handles > 0
                && summary_length + handle_marker.chars().count() > MAX_SUMMARY_CHARS);
        if omitted_handles == 0 && !summary_was_truncated {
            self.summary = original_summary;
            return false;
        }

        let marker = match (summary_was_truncated, omitted_handles) {
            (true, 0) => "\n[branch result summary truncated]".to_owned(),
            (false, count) => format!("\n[{count} branch result handles omitted]"),
            (true, count) => {
                format!("\n[summary truncated; {count} branch result handles omitted]")
            }
        };
        let prefix_limit = MAX_SUMMARY_CHARS.saturating_sub(marker.chars().count());
        let prefix = summary.chars().take(prefix_limit).collect::<String>();
        self.summary = Some(format!("{prefix}{marker}"));
        true
    }
}

impl Session {
    /// Compacts every branch result before a session graph is persisted.
    ///
    /// # Parameters
    /// - `self`: Session containing durable branch results.
    ///
    /// # Returns
    /// Number of branch results that required compaction.
    pub fn compact_branch_results(&mut self) -> usize {
        self.branches
            .iter_mut()
            .filter_map(|branch| branch.result.as_mut())
            .map(|result| usize::from(result.compact_for_storage()))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{BranchResult, MAX_RESULT_HANDLE_CHARS, MAX_RESULT_HANDLES, MAX_SUMMARY_CHARS};
    use crate::BranchStatus;
    use scriberig_core::EntityId;

    type SampleBranch = BranchResult;

    /// Creates a branch result with caller-supplied summary and handles.
    ///
    /// - `summary`: Optional concise result summary.
    /// - `result_handles`: Stable handles produced by the branch.
    ///
    /// # Returns
    /// A completed branch result with the supplied summary and handles.
    fn sample(summary: Option<String>, result_handles: Vec<String>) -> SampleBranch {
        BranchResult {
            id: EntityId::from_bytes([1; 16]),
            branch: EntityId::from_bytes([2; 16]),
            status: BranchStatus::Completed,
            summary,
            result_kind: Some("agent".to_owned()),
            candidate_context: None,
            references: Vec::new(),
            result_handles,
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        }
    }

    /// Keeps compact results unchanged and marks all durable truncation.
    #[test]
    fn branch_result_compaction_bounds_summary_and_handles_with_a_marker() {
        let mut compact = sample(Some("done".to_owned()), vec!["feature:124".to_owned()]);
        assert!(!compact.compact_for_storage());
        assert_eq!(compact.summary.as_deref(), Some("done"));
        assert_eq!(compact.result_handles, ["feature:124"]);

        let handles = (0..40)
            .map(|index| format!("feature:{index}"))
            .chain(std::iter::once("x".repeat(300)))
            .collect();
        let mut oversized = sample(Some("s".repeat(3_000)), handles);
        assert!(oversized.compact_for_storage());
        assert_eq!(oversized.result_handles.len(), MAX_RESULT_HANDLES);
        assert!(
            oversized
                .result_handles
                .iter()
                .all(|handle| handle.chars().count() <= MAX_RESULT_HANDLE_CHARS)
        );
        assert!(oversized.summary.as_ref().expect("summary").chars().count() <= MAX_SUMMARY_CHARS);
        assert!(
            oversized
                .summary
                .as_ref()
                .expect("summary")
                .contains("summary truncated; 9 branch result handles omitted")
        );
        assert!(!oversized.compact_for_storage());

        let mut boundary = sample(
            Some("b".repeat(MAX_SUMMARY_CHARS)),
            (0..=MAX_RESULT_HANDLES)
                .map(|index| format!("feature:{index}"))
                .collect(),
        );
        assert!(boundary.compact_for_storage());
        let summary = boundary.summary.as_ref().expect("bounded summary");
        assert!(summary.contains("summary truncated; 1 branch result handles omitted"));
        assert!(summary.chars().count() <= MAX_SUMMARY_CHARS);
    }
}
