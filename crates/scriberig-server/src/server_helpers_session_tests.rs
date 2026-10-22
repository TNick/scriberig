//! Verifies the server preserves safe durable plan projections.

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, ModelAssignment, Plan, PlanDependency, PlanNode, PlanNodeStatus, Session,
    SessionExecutionSelection,
};

/// Preserves assignments and exact fulfilled results in session responses.
#[test]
fn session_response_preserves_plan_dependencies_and_safe_assignments() {
    let session_id = EntityId::from_bytes([41; 16]);
    let root = EntityId::from_bytes([42; 16]);
    let source = EntityId::from_bytes([43; 16]);
    let dependent = EntityId::from_bytes([44; 16]);
    let result = EntityId::from_bytes([45; 16]);
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(root, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![Plan {
            id: EntityId::from_bytes([46; 16]),
            root_branch: root,
            nodes: vec![
                PlanNode {
                    id: source,
                    parent: Some(scriberig_session::PlanNodeParent::Branch(root)),
                    title: "source".to_owned(),
                    purpose: "prepare API".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    status: PlanNodeStatus::Completed,
                    branch: Some(EntityId::from_bytes([47; 16])),
                    result: Some(result),
                    superseded_by: None,
                    assignment: Some(ModelAssignment {
                        provider_id: "provider-a".to_owned(),
                        model_id: "model-a".to_owned(),
                        thinking_level: Some("focused".to_owned()),
                    }),
                    claim: None,
                    claim_generation: 0,
                },
                PlanNode {
                    id: dependent,
                    parent: Some(scriberig_session::PlanNodeParent::Branch(root)),
                    title: "consumer".to_owned(),
                    purpose: "consume API".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    status: PlanNodeStatus::Planned,
                    branch: None,
                    result: None,
                    superseded_by: None,
                    assignment: None,
                    claim: None,
                    claim_generation: 0,
                },
            ],
            dependencies: vec![PlanDependency {
                prerequisite: source,
                dependent,
                fulfilled_by: Some(result),
            }],
            coordinator: Some(root),
            coordinator_cursor: Some(dependent),
            coordinator_claim: None,
            coordinator_generation: 0,
            session: None,
            anchor: None,
            goal: String::new(),
            revision: 0,
            status: scriberig_session::PlanStatus::Draft,
            history: Vec::new(),
            root_nodes: vec![source, dependent],
        }],
        execution: SessionExecutionSelection::default(),
    };

    let response = super::session_response(session).expect("session response");
    let plan = &response.plans[0];
    assert_eq!(
        plan.dependencies[0].fulfilled_by.as_deref(),
        Some(result.to_string().as_str())
    );
    assert_eq!(plan.coordinator.as_deref(), Some(root.to_string().as_str()));
    assert_eq!(plan.revision, 0);
    assert_eq!(plan.status, scriberig_protocol::SessionPlanStatus::Draft);
    let assignment = plan.nodes[0].assignment.as_ref().unwrap();
    assert_eq!(assignment.provider_id, "provider-a");
    assert_eq!(assignment.model_id, "model-a");
}
