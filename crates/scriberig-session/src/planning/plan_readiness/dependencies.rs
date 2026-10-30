//! Checks exact successful branch-result contracts for plan dependencies.

use super::super::{Plan, PlanDependency, PlanNodeStatus};

impl Plan {
    /// Validates one exact successful result against its prerequisite contract.
    ///
    /// # Parameters
    /// - `self`: Plan defining prerequisite state and result policy.
    /// - `edge`: Dependency edge to evaluate.
    /// - `results`: Durable results in the same session snapshot.
    ///
    /// `true` when the edge points to the prerequisite's accepted result.
    /// # Returns
    /// Whether the prerequisite has its exact successful result.
    pub(in crate::planning) fn dependency_satisfied(
        &self,
        edge: &PlanDependency,
        results: &[crate::BranchResult],
    ) -> bool {
        let Some(result_id) = edge.fulfilled_by else {
            return false;
        };
        let Some(result) = results.iter().find(|item| item.id == result_id) else {
            return false;
        };
        let Some(prerequisite) = self.nodes.iter().find(|item| item.id == edge.prerequisite) else {
            return false;
        };
        if result.status != crate::BranchStatus::Completed
            || prerequisite.status != PlanNodeStatus::Completed
            || prerequisite.result != Some(result_id)
            || prerequisite.branch != Some(result.branch)
        {
            return false;
        }
        prerequisite
            .expected_result
            .as_ref()
            .is_none_or(|contract| {
                result.result_kind.as_deref() == Some(contract.kind.as_str())
                    && (!contract.requires_handle
                        || !result.result_handles.is_empty()
                        || !result.references.is_empty())
            })
    }
}
