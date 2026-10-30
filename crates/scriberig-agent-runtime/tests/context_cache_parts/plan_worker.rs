//! Covers plan-worker context isolation.

use super::*;

/// Verifies plan workers receive exact anchor and result context without raw
/// sibling or prerequisite transcripts.
#[test]
#[allow(clippy::too_many_lines)] // The scenario compares all inherited and excluded context.
fn plan_worker_context_isolated_to_anchor_and_exact_results() {
    let root_id = EntityId::from_bytes([211; 16]);
    let prerequisite_branch_id = EntityId::from_bytes([212; 16]);
    let sibling_id = EntityId::from_bytes([213; 16]);
    let worker_branch_id = EntityId::from_bytes([214; 16]);
    let prerequisite_id = EntityId::from_bytes([215; 16]);
    let worker_id = EntityId::from_bytes([216; 16]);
    let result_id = EntityId::from_bytes([217; 16]);
    let session_id = EntityId::from_bytes([218; 16]);
    let plan_id = EntityId::from_bytes([219; 16]);
    let anchor_turn_id = EntityId::from_bytes([220; 16]);
    let late_turn_id = EntityId::from_bytes([221; 16]);

    let mut root = Branch::new(root_id, "root").expect("root branch");
    root.turns = vec![
        Turn {
            id: anchor_turn_id,
            ordinal: 1,
            human: Some(HumanInput {
                text: "selected anchor question".to_owned(),
            }),
            agent: Some(AgentOutput {
                text: "selected anchor answer".to_owned(),
            }),
            tool_invocation: None,
            tool_result: None,
        },
        Turn {
            id: late_turn_id,
            ordinal: 2,
            human: Some(HumanInput {
                text: "later root transcript excluded".to_owned(),
            }),
            agent: None,
            tool_invocation: None,
            tool_result: None,
        },
    ];
    let mut prerequisite =
        Branch::new(prerequisite_branch_id, "prerequisite").expect("prerequisite branch");
    prerequisite.turns.push(Turn {
        id: EntityId::from_bytes([222; 16]),
        ordinal: 1,
        human: Some(HumanInput {
            text: "PRIVATE_DEPENDENCY_TRANSCRIPT".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    prerequisite.result = Some(BranchResult {
        id: result_id,
        branch: prerequisite_branch_id,
        status: BranchStatus::Completed,
        summary: Some("public compact dependency result".to_owned()),
        result_kind: Some("api-contract".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec!["entity:api-contract".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    let mut sibling = Branch::new(sibling_id, "sibling").expect("sibling branch");
    sibling.turns.push(Turn {
        id: EntityId::from_bytes([223; 16]),
        ordinal: 1,
        human: Some(HumanInput {
            text: "PRIVATE_SIBLING_TRANSCRIPT".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    let mut worker = Branch::new_child(worker_branch_id, root_id, "worker", "implement consumer")
        .expect("worker branch");
    root.children.push(worker_branch_id);
    worker.turns.push(Turn {
        id: EntityId::from_bytes([224; 16]),
        ordinal: 1,
        human: Some(HumanInput {
            text: "worker local history".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    let anchor = ContinuationAnchor::AssistantOutput {
        branch: root_id,
        turn: anchor_turn_id,
    };
    let session = Session {
        id: session_id,
        branches: vec![root, prerequisite, sibling, worker],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let plan = Plan {
        id: plan_id,
        session: Some(session_id),
        anchor: Some(anchor),
        goal: "implement consumer".to_owned(),
        revision: 1,
        status: PlanStatus::Running,
        root_branch: root_id,
        nodes: vec![
            PlanNode {
                id: prerequisite_id,
                parent: Some(PlanNodeParent::Branch(root_id)),
                title: "publish API".to_owned(),
                purpose: "define API".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                status: PlanNodeStatus::Completed,
                branch: Some(prerequisite_branch_id),
                result: Some(result_id),
                superseded_by: None,
                assignment: None,
                claim: None,
                claim_generation: 1,
            },
            PlanNode {
                id: worker_id,
                parent: Some(PlanNodeParent::Branch(root_id)),
                title: "implement consumer".to_owned(),
                purpose: "use the published API".to_owned(),
                expected_result: None,
                semantic_scope: vec!["entity:consumer-module".to_owned()],
                status: PlanNodeStatus::Running,
                branch: Some(worker_branch_id),
                result: None,
                superseded_by: None,
                assignment: None,
                claim: Some(WorkerClaim {
                    id: EntityId::from_bytes([225; 16]),
                    node: worker_id,
                    branch: worker_branch_id,
                    owner: "test-worker".to_owned(),
                    generation: 1,
                    assignment: None,
                }),
                claim_generation: 1,
            },
        ],
        dependencies: vec![PlanDependency {
            prerequisite: prerequisite_id,
            dependent: worker_id,
            fulfilled_by: Some(result_id),
        }],
        coordinator: None,
        coordinator_cursor: Some(worker_id),
        coordinator_claim: None,
        coordinator_generation: 0,
        root_nodes: vec![prerequisite_id, worker_id],
        history: Vec::new(),
    };
    let claim = plan.nodes[1].claim.as_ref().expect("worker claim");

    let context = PlanWorkerContext::from_plan(&session, &plan, &plan.nodes[1], claim)
        .expect("compile exact worker context");
    assert_eq!(
        context.inputs.branch_role,
        BranchRole::NonRoot {
            parent: root_id.to_string(),
        }
    );
    assert!(context.inputs.project_scope.is_none());
    let compiled = context.compiled.canonical;

    assert!(compiled.contains("scriberig.context-component.core-invariants@"));
    assert!(compiled.contains("scriberig.context-component.non-root-handoff@"));
    assert!(compiled.contains("selected anchor question"));
    assert!(compiled.contains("selected anchor answer"));
    assert!(compiled.contains("public compact dependency result"));
    assert!(compiled.contains("entity:api-contract"));
    assert!(compiled.contains("entity:consumer-module"));
    assert!(compiled.contains("worker local history"));
    assert!(!compiled.contains("later root transcript excluded"));
    assert!(!compiled.contains("PRIVATE_DEPENDENCY_TRANSCRIPT"));
    assert!(!compiled.contains("PRIVATE_SIBLING_TRANSCRIPT"));
}
