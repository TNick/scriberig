//! Verifies plan execution selections render the exact stored upstream IDs.

use super::test_support::{sample_plan, session_with_plans};
use super::*;

/// Preserves the provider, model, and thinking IDs on the selected plan node.
#[test]
fn plan_assignment_details_project_exact_upstream_selection() {
    let plan_id = EntityId::from_bytes([81; 16]);
    let branch = EntityId::from_bytes([82; 16]);
    let node = EntityId::from_bytes([83; 16]);
    let mut plan = sample_plan(plan_id, branch, node, EntityId::from_bytes([84; 16]));
    plan.nodes[0].assignment = Some(scriberig_session::ModelAssignment {
        provider_id: "provider-from-upstream".to_owned(),
        model_id: "model-from-upstream".to_owned(),
        thinking_level: Some("high".to_owned()),
    });
    let state = TuiState {
        session_snapshot: Some(session_with_plans(vec![plan])),
        plan_cursor: Some(PlanSelection {
            plan: plan_id,
            node: Some(node),
        }),
        ..TuiState::default()
    };

    let (_, details, _) = plan_tree(&state).expect("plan tree projection");

    assert!(
        details.contains("Assignment: provider-from-upstream / model-from-upstream / high"),
        "the view must display the exact upstream-resolved assignment IDs: {details}"
    );
}
