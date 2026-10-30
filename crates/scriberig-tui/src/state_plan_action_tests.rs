//! Verifies plan commands emit revision-checked shared application operations.

use super::*;
use scriberig_session::{ContinuationAnchor, Plan, PlanNode, PlanNodeParent, PlanNodeStatus};

/// Verifies next, child, retry, and waiver commands preserve exact selected identities.
#[test]
fn plan_commands_emit_shared_revision_checked_actions() {
    let session = scriberig_core::EntityId::from_bytes([31; 16]);
    let plan_id = scriberig_core::EntityId::from_bytes([32; 16]);
    let branch = scriberig_core::EntityId::from_bytes([33; 16]);
    let node = scriberig_core::EntityId::from_bytes([34; 16]);
    let gate = scriberig_core::EntityId::from_bytes([35; 16]);
    let plan = plan_with_node(session, plan_id, branch, node);
    let expected_revision = plan.revision;

    let effects = submit_plan_command(
        session,
        branch,
        plan.clone(),
        node,
        "/plan next Follow up: inspect the result",
    );
    let [Effect::AddPlanNextStep(request)] = effects.as_slice() else {
        panic!("next-step command should emit a shared request");
    };
    assert_eq!(request.session, session);
    assert_eq!(request.plan, plan_id);
    assert_eq!(request.expected_revision, expected_revision);
    assert_eq!(request.title, "Follow up");
    assert_eq!(request.purpose, "inspect the result");

    let effects = submit_plan_command(
        session,
        branch,
        plan.clone(),
        node,
        "/plan child Detail: inspect one result",
    );
    let [Effect::DecomposePlanNode(request)] = effects.as_slice() else {
        panic!("child command should emit a shared request");
    };
    assert_eq!(request.session, session);
    assert_eq!(request.plan, plan_id);
    assert_eq!(request.parent, node);
    assert_eq!(request.expected_revision, expected_revision);
    assert!(request.required);
    assert_eq!(request.children[0].title, "Detail");

    let effects = submit_plan_command(session, branch, plan.clone(), node, "/plan retry");
    let [Effect::RetryPlanNode(request)] = effects.as_slice() else {
        panic!("retry command should emit a shared request");
    };
    assert_eq!(request.session, session);
    assert_eq!(request.plan, plan_id);
    assert_eq!(request.node, node);
    assert_eq!(request.expected_revision, expected_revision);

    let effects = submit_plan_command(
        session,
        branch,
        plan,
        node,
        &format!("/plan waive {gate} reviewed exception"),
    );
    let [Effect::MutatePlanSemantics(request)] = effects.as_slice() else {
        panic!("waiver command should emit a shared request");
    };
    assert_eq!(request.session, session);
    assert_eq!(request.plan, plan_id);
    assert_eq!(request.expected_revision, expected_revision);
    assert_eq!(
        request.mutation,
        scriberig_app::PlanSemanticMutation::WaiveGate {
            gate,
            reason: "reviewed exception".to_owned(),
        }
    );
}

/// Builds and submits one plan command against a selected plan-node snapshot.
///
/// - `session_id`: Durable session identity.
/// - `branch`: Structural root branch.
/// - `plan`: Canonical plan projection.
/// - `node`: Selected node identity.
/// - `command`: Slash command submitted to the reducer.
///
/// Effects emitted by the shared command reducer.
/// # Returns
/// The plan action effect.
fn submit_plan_command(
    session_id: scriberig_core::EntityId,
    branch: scriberig_core::EntityId,
    plan: Plan,
    node: scriberig_core::EntityId,
    command: &str,
) -> Vec<Effect> {
    let original_plan = plan.clone();
    let plan_id = plan.id;
    let session = scriberig_session::Session {
        id: session_id,
        branches: vec![scriberig_session::Branch::new(branch, "Root").expect("branch")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut state = TuiState {
        primary_view: crate::selection::PrimaryView::Plan,
        session_snapshot: Some(session),
        plan_cursor: Some(crate::selection::PlanSelection {
            plan: plan_id,
            node: Some(node),
        }),
        ..TuiState::default()
    };
    state.session_composer_focused = true;
    state.session_composer = Some(crate::state::SessionComposerState {
        session: session_id,
        branch: Some(branch),
        text: command.to_owned(),
        pending_prompt: None,
        error: None,
    });
    let effects = reduce(&mut state, Action::InputSubmitted);
    assert_eq!(
        state.session_snapshot.as_ref().unwrap().plans[0],
        original_plan,
        "plan commands must leave canonical plan state unchanged"
    );
    effects
}

/// Builds a plan with one durable node for action-command assertions.
///
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity.
/// - `branch`: Structural root branch.
/// - `node`: Stable plan-node identity.
///
/// A valid single-node draft plan.
/// # Returns
/// The canonical plan projection.
fn plan_with_node(
    session: scriberig_core::EntityId,
    plan: scriberig_core::EntityId,
    branch: scriberig_core::EntityId,
    node: scriberig_core::EntityId,
) -> Plan {
    let mut plan = Plan::new(
        plan,
        session,
        ContinuationAnchor::BranchHead { branch },
        "shared plan actions",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node,
        parent: Some(PlanNodeParent::Branch(branch)),
        title: "Selected work".to_owned(),
        purpose: "Run selected work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Failed,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    })
    .expect("node");
    plan
}
