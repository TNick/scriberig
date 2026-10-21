//! Projects durable plan topology and dependency data into safe Markdown.

use crate::state::TuiState;

/// Appends exact plan identity, assignment, coordinator, and dependency data.
///
/// - `output`: Markdown receiving durable plan details.
/// - `state`: Session snapshot containing the canonical plan values.
pub(super) fn append_plan_graphs(output: &mut String, state: &TuiState) {
    let Some(session) = &state.session_snapshot else {
        return;
    };
    for plan in session.plans.iter().take(32) {
        super::super::common::append_value(
            output,
            &format!(
                "Plan {} · revision {} · {:?}",
                plan.id, plan.revision, plan.status
            ),
        );
        super::super::common::append_value(output, &format!("Goal: {}", plan.goal));
        if let Some(claim) = &plan.coordinator_claim {
            super::super::common::append_value(output, &format!("Coordinator claim: {}", claim.id));
        }
        if let Some(cursor) = plan.coordinator_cursor {
            super::super::common::append_value(output, &format!("Coordinator cursor: {cursor}"));
        }
        for node in plan.nodes.iter().take(super::super::common::MAX_ROWS) {
            if let Some(branch) = node.branch {
                super::super::common::append_value(
                    output,
                    &format!("{} · branch: {branch}", node.title),
                );
            }
            if let Some(result) = node.result {
                super::super::common::append_value(
                    output,
                    &format!("{} · BranchResult: {result}", node.title),
                );
            }
        }
        for edge in plan
            .dependencies
            .iter()
            .take(super::super::common::MAX_ROWS)
        {
            super::super::common::append_value(
                output,
                &format!(
                    "Dependency edge: {} → {} · fulfilled by {}",
                    edge.prerequisite,
                    edge.dependent,
                    edge.fulfilled_by
                        .map_or_else(|| "pending".to_owned(), |id| id.to_string())
                ),
            );
        }
    }
}
