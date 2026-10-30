//! Verifies shared plan decomposition creates structural children and gates.

use super::*;

/// Decomposes a node without inventing dependencies between its children.
#[test]
fn application_decomposition_creates_required_child_gates_once() {
    let service = ApplicationService::new(Fixture);
    let context = session_context();
    let session = EntityId::from_bytes([151; 16]);
    service
        .start_agent(&context, session, "decompose plan")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("session");
    let root = state.branches[0].id;
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead { branch: root },
                goal: "decompose work".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "parent".to_owned(),
                    parent_index: None,
                    title: "Parent work".to_owned(),
                    purpose: "coordinate the child tasks".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    let parent = plan.nodes[0].id;
    let running = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
            },
        )
        .expect("start plan");
    let decomposed = service
        .decompose_plan_node(
            &context,
            &PlanDecomposeRequest {
                session,
                plan: plan.id,
                parent,
                expected_revision: running.revision,
                children: vec![PlanChildDraft {
                    title: "Child work".to_owned(),
                    purpose: "complete one required part".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                required: true,
            },
        )
        .expect("decompose parent");

    assert_eq!(decomposed.revision, running.revision + 1);
    assert_eq!(decomposed.dependencies.len(), 0);
    let child = decomposed
        .nodes
        .iter()
        .find(|node| node.id != parent)
        .expect("child node");
    assert_eq!(
        child.parent,
        Some(scriberig_session::PlanNodeParent::Node(parent))
    );
    assert_eq!(decomposed.gates.len(), 1);
    assert_eq!(
        decomposed.gates[0].requirement,
        scriberig_session::PlanGateRequirement::RequiredChildCompleted { child: child.id }
    );
}
