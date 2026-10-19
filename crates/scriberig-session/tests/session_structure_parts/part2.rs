//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies branch structure is navigable.
#[test]
fn branch_structure_is_navigable() {
    let id = EntityId::from_bytes([1; 16]);
    let mut branch = Branch::new(id, "work").expect("branch");
    branch.touch(EntityId::from_bytes([8; 16]));
    branch.push_turn(Turn {
        id: EntityId::from_bytes([2; 16]),
        ordinal: 99,
        human: None,
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    assert_eq!(branch.turns[0].ordinal, 0);
    branch
        .transition(BranchStatus::Completed)
        .expect("complete");
    assert_eq!(
        branch.transition(BranchStatus::Active),
        Err(SessionError::InvalidTransition {
            from: BranchStatus::Completed,
            to: BranchStatus::Active
        })
    );
    let session = Session {
        id: EntityId::from_bytes([3; 16]),
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };
    assert_eq!(session.branches.len(), 1);
}

/// Checks that child insertion maintains both directions of the graph edge.
#[test]
fn child_branch_insertion_is_validated_and_linked() {
    let root_id = EntityId::from_bytes([10; 16]);
    let child_id = EntityId::from_bytes([11; 16]);
    let mut session = Session {
        id: EntityId::from_bytes([12; 16]),
        branches: vec![Branch::new(root_id, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    };

    session
        .add_child_branch(child_id, root_id, "child", "inspect")
        .expect("child");
    assert_eq!(session.branches[0].id, root_id);
    assert_eq!(session.branches[0].children, vec![child_id]);
    assert_eq!(session.branches[1].parent, Some(root_id));
    assert_eq!(session.branches[1].purpose.as_deref(), Some("inspect"));
    assert_eq!(
        session.add_child_branch(child_id, root_id, "duplicate", "again"),
        Err(SessionError::DuplicateBranch)
    );
    assert_eq!(
        session.add_child_branch(
            EntityId::from_bytes([13; 16]),
            EntityId::from_bytes([14; 16]),
            "orphan",
            "none"
        ),
        Err(SessionError::MissingParent)
    );
}

/// Keeps dependency readiness tied to explicit results and rejects cycles.
#[test]
fn plan_dependencies_require_results_and_remain_acyclic() {
    let parser = EntityId::from_bytes([22; 16]);
    let docs = EntityId::from_bytes([23; 16]);
    let integration = EntityId::from_bytes([24; 16]);
    let parser_branch = EntityId::from_bytes([25; 16]);
    let docs_branch = EntityId::from_bytes([26; 16]);
    let parser_result = EntityId::from_bytes([27; 16]);
    let docs_result = EntityId::from_bytes([28; 16]);
    let mut plan = plan_fixture();

    plan.add_dependency(parser, integration)
        .expect("parser dependency");
    plan.add_dependency(docs, integration)
        .expect("docs dependency");
    assert_eq!(plan.ready_nodes_from_results(&[]), vec![parser, docs]);
    assert_eq!(
        plan.add_dependency(integration, parser),
        Err(PlanError::DependencyCycle)
    );
    assert_eq!(
        plan.add_dependency(parser, parser),
        Err(PlanError::SelfDependency)
    );
    assert_eq!(
        plan.add_dependency(EntityId::from_bytes([99; 16]), parser),
        Err(PlanError::MissingNode)
    );
    assert_eq!(
        plan.dependencies,
        vec![
            scriberig_session::PlanDependency {
                prerequisite: parser,
                dependent: integration,
                fulfilled_by: None,
            },
            scriberig_session::PlanDependency {
                prerequisite: docs,
                dependent: integration,
                fulfilled_by: None,
            },
        ]
    );

    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([25; 16]), "coordinator")
        .expect("claim coordinator");
    let parser_claim = EntityId::from_bytes([26; 16]);
    let docs_claim = EntityId::from_bytes([27; 16]);
    plan.start_node(parser, parser_branch, parser_claim, "parser", &[])
        .expect("start parser");
    plan.start_node(docs, docs_branch, docs_claim, "docs", &[])
        .expect("start docs concurrently");
    let parser_output = branch_result(parser_result, parser_branch, None);
    let docs_output = branch_result(docs_result, docs_branch, None);
    plan.complete_node(parser, parser_claim, &parser_output)
        .expect("parser result");
    assert_eq!(
        plan.ready_nodes_from_results(std::slice::from_ref(&parser_output)),
        Vec::<EntityId>::new()
    );
    assert_eq!(
        plan.nodes
            .iter()
            .find(|node| node.id == integration)
            .unwrap()
            .status,
        PlanNodeStatus::Planned
    );

    plan.complete_node(docs, docs_claim, &docs_output)
        .expect("docs result");
    assert_eq!(
        plan.ready_nodes_from_results(&[parser_output.clone(), docs_output.clone()]),
        vec![integration]
    );
    assert_eq!(plan.dependencies[0].fulfilled_by, Some(parser_result));
    let assignment = ModelAssignment {
        profile: "fast".to_owned(),
        model: "model-a".to_owned(),
        thinking_level: None,
    };
    plan.nodes
        .iter_mut()
        .find(|node| node.id == integration)
        .unwrap()
        .assignment = Some(assignment);
    assert_eq!(plan.nodes.len(), 3);
}
