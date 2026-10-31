//! Holds additional focused scenarios for the parent test module.

use super::*;

/// Shows failed and accepted attempts by identity without rendering child transcript text.
#[test]
fn selected_node_details_expose_attempt_history_as_safe_references() {
    let plan_id = EntityId::from_bytes([131; 16]);
    let root_branch = EntityId::from_bytes([132; 16]);
    let node_id = EntityId::from_bytes([133; 16]);
    let failed_branch = EntityId::from_bytes([134; 16]);
    let accepted_branch = EntityId::from_bytes([135; 16]);
    let accepted_result = EntityId::from_bytes([136; 16]);

    let mut plan = sample_plan(
        plan_id,
        root_branch,
        node_id,
        EntityId::from_bytes([137; 16]),
    );
    let mut failed = node_with_status(node_id, None, PlanNodeStatus::Failed);
    failed.branch = Some(failed_branch);
    failed.claim_generation = 1;
    let mut accepted = node_with_status(node_id, None, PlanNodeStatus::Completed);
    accepted.branch = Some(accepted_branch);
    accepted.result = Some(accepted_result);
    accepted.claim_generation = 2;
    plan.nodes = vec![accepted];
    plan.dependencies.clear();
    plan.root_nodes = vec![node_id];
    plan.history.push(PlanRevision {
        revision: 3,
        goal: plan.goal.clone(),
        status: PlanStatus::Running,
        nodes: vec![failed],
        dependencies: Vec::new(),
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        reason: None,
    });
    let (rows, session) = plan_rows(plan);

    let details = selected_details(&session, &rows, plan_id, Some(node_id));

    assert!(details.contains("## Worker attempts"), "{details}");
    assert!(
        details.contains(&format!("Generation 1 · Failed · branch {failed_branch}")),
        "{details}"
    );
    assert!(
        details.contains(&format!(
            "Generation 2 · Completed · branch {accepted_branch} · result {accepted_result}"
        )),
        "{details}"
    );
    assert!(
        !details.contains("child transcript"),
        "attempt details must not include branch transcript: {details}"
    );
}
