//! Verifies shared TUI plan operations delegate to the application session backend.

use super::*;

/// Verifies node assignment effects use the shared application backend.
#[test]
fn delegates_plan_node_assignment_to_shared_session_backend() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let session = EntityId::from_bytes([93; 16]);
    let plan = EntityId::from_bytes([94; 16]);
    let node = EntityId::from_bytes([95; 16]);
    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::SetPlanNodeAssignment(scriberig_app::PlanAssignmentRequest {
            session,
            plan,
            node,
            expected_revision: 2,
            assignment: None,
        }),
    );
    assert!(matches!(action, Action::PlanUpdated(value) if value.id == plan));
}

/// Verifies replan effects use the shared application backend.
#[test]
fn delegates_structured_plan_revision_to_shared_session_backend() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let session = EntityId::from_bytes([97; 16]);
    let plan = EntityId::from_bytes([98; 16]);
    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::Replan(scriberig_app::PlanReplanRequest {
            session,
            plan,
            expected_revision: 1,
            additions: Vec::new(),
            dependencies: Vec::new(),
            removed_dependencies: Vec::new(),
            superseded: Vec::new(),
            assignment_changes: Vec::new(),
            preserved_terminal_nodes: Vec::new(),
            reason: "revise scope".to_owned(),
        }),
    );
    assert!(matches!(action, Action::PlanUpdated(value) if value.id == plan));
}

/// Routes plan creation effects through the application service adapter.
#[test]
fn executes_plan_creation_through_shared_service() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let request = scriberig_app::PlanCreateRequest {
        session: EntityId::from_bytes([92; 16]),
        anchor: scriberig_session::ContinuationAnchor::BranchHead {
            branch: EntityId::from_bytes([93; 16]),
        },
        goal: "Improve parser".to_owned(),
        nodes: Vec::new(),
        dependencies: Vec::new(),
    };

    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::CreatePlan(request),
    );

    assert!(matches!(
        action,
        Action::PlanUpdated(plan) if plan.goal == "Improve parser"
    ));
}
