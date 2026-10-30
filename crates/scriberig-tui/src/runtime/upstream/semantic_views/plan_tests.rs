//! Verifies stable plan projection and separate dependency-edge presentation.

use super::*;
use scriberig_session::PlanNode;

/// Builds a deterministic plan with structural and dependency relations diverging.
///
/// - `plan_id`: Durable plan identity.
/// - `branch`: Structural parent branch identity.
/// - `first`: First durable node identity.
/// - `second`: Second durable node identity.
///
/// # Returns
/// A plan with one structural edge and a reverse dependency edge.
fn sample_plan(plan_id: EntityId, branch: EntityId, first: EntityId, second: EntityId) -> Plan {
    Plan {
        id: plan_id,
        source_snapshot: None,
        session: Some(EntityId::from_bytes([9; 16])),
        anchor: None,
        goal: "Verify graph projection".to_owned(),
        revision: 4,
        status: scriberig_session::PlanStatus::Running,
        root_branch: branch,
        nodes: vec![
            node(first, None),
            node(second, Some(PlanNodeParent::Node(first))),
        ],
        dependencies: vec![scriberig_session::PlanDependency {
            prerequisite: second,
            dependent: first,
            fulfilled_by: None,
        }],
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        coordinator: Some(branch),
        coordinator_cursor: Some(first),
        coordinator_claim: None,
        coordinator_generation: 1,
        root_nodes: vec![first],
        history: Vec::new(),
    }
}

/// Builds a planned node with a stable structural parent.
///
/// - `id`: Durable node identity.
/// - `parent`: Structural parent relation.
///
/// # Returns
/// A ready plan node for projection assertions.
fn node(id: EntityId, parent: Option<PlanNodeParent>) -> PlanNode {
    PlanNode {
        id,
        parent,
        title: format!("Work {id}"),
        purpose: "Keep plan semantics intact".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: scriberig_session::PlanNodeStatus::Ready,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}

/// Verifies the tree uses structural parents while Markdown shows DAG edges.
#[test]
fn structural_tree_and_dependency_edges_remain_distinct() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let mut plan = sample_plan(plan_id, branch, first, second);
    plan.nodes[0].superseded_by = Some(second);
    plan.nodes[0].branch = Some(branch);
    plan.nodes[0].claim_generation = 1;
    let gate = EntityId::from_bytes([5; 16]);
    let evidence = EntityId::from_bytes([6; 16]);
    let evidence_entity = EntityId::from_bytes([7; 16]);
    plan.gates.push(scriberig_session::PlanGate {
        id: gate,
        owner: first,
        phase: scriberig_session::PlanGatePhase::Entry,
        requirement: scriberig_session::PlanGateRequirement::ValidationPassed {
            profile: Some("tests".to_owned()),
        },
        waiver: None,
    });
    plan.evidence
        .push(scriberig_session::PlanEvidenceAssociation {
            id: evidence,
            owner: scriberig_session::PlanEvidenceOwner::Gate(gate),
            role: scriberig_session::PlanEvidenceRole::Contradicts,
            evidence: scriberig_session::PlanEvidenceReference {
                kind: scriberig_session::PlanEvidenceKind::ValidationResult,
                reference: scriberig_session::PlanSemanticReference {
                    entity: evidence_entity,
                    revision: None,
                    snapshot: None,
                },
                attributes: std::collections::BTreeMap::from([
                    ("profile".to_owned(), "tests".to_owned()),
                    ("passed".to_owned(), "false".to_owned()),
                ]),
            },
            note: Some("validation did not pass".to_owned()),
            actor: None,
        });
    plan.history.push(scriberig_session::PlanRevision {
        revision: 3,
        goal: "Previous graph goal".to_owned(),
        status: scriberig_session::PlanStatus::Paused,
        nodes: vec![node(first, None)],
        dependencies: Vec::new(),
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        reason: Some("Split the root work into focused nodes".to_owned()),
    });
    let session = scriberig_session::Session {
        id: EntityId::from_bytes([9; 16]),
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan.clone()],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let rows = rows_for_session(&session).expect("shared plan views");
    let first_row = rows
        .iter()
        .find(|row| row.selection.node == Some(first))
        .expect("first node row");
    let second_row = rows
        .iter()
        .find(|row| row.selection.node == Some(second))
        .expect("second node row");
    assert_eq!(second_row.parent_id.as_deref(), Some(first_row.id.as_str()));

    let details = plan_details(
        &session,
        Some(PlanSelection {
            plan: plan_id,
            node: Some(second),
        }),
        &rows,
    );
    assert!(details.contains(&format!("{second} → {first}")));
    assert!(details.contains("Replan reason: Split the root work into focused nodes"));
    let node_details = plan_details(
        &session,
        Some(PlanSelection {
            plan: plan_id,
            node: Some(first),
        }),
        &rows,
    );
    assert!(node_details.contains(&format!("Superseded by node: {second}")));
    assert!(node_details.contains("Direct status: Ready"));
    assert!(node_details.contains("Effective status: Ready"));
    assert!(node_details.contains("### Gate details"));
    assert!(node_details.contains("### Evidence references"));
    assert!(node_details.contains("### Attempts"));
    assert!(node_details.contains(&format!("Entry gate {gate} · Failed")));
    assert!(node_details.contains(&format!("entity {evidence_entity}")));
    assert!(node_details.contains("validation did not pass"));
    assert!(node_details.contains(&format!("branch {branch}")));
}

/// Matches TUI node views to the shared server, protocol, and web fixture.
#[test]
fn tui_plan_views_match_shared_cross_surface_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/agent_session_plan_conformance.json"
    ))
    .expect("shared plan fixture");
    let expected_plan = &fixture["session"]["plans"][0];
    let root = EntityId::from_bytes([1; 16]);
    let mut plan_value = expected_plan.clone();
    plan_value["root_branch"] = serde_json::json!(root.to_string());
    plan_value["coordinator"] = serde_json::json!(root.to_string());
    plan_value["coordinator_claim"]["id"] =
        serde_json::json!(EntityId::from_bytes([17; 16]).to_string());
    plan_value["anchor"]["BranchHead"]["branch"] = serde_json::json!(root.to_string());
    for node in plan_value["nodes"].as_array_mut().expect("fixture nodes") {
        if node["parent"] == "branch-root-1" {
            node["parent"] = serde_json::json!(root.to_string());
        }
    }
    let response: scriberig_protocol::SessionPlan =
        serde_json::from_value(plan_value).expect("protocol plan fixture");
    let plan =
        crate::service::session_plan_from_response(response).expect("restore shared plan fixture");
    let session = fixture_session(plan);
    let rows = rows_for_session(&session).expect("build shared semantic views");
    let expected_nodes = expected_plan["nodes"].as_array().expect("fixture nodes");
    let plan_view = rows.first().expect("plan header row").plan.clone();

    assert_eq!(
        plan_view.id.to_string(),
        expected_plan["id"].as_str().unwrap()
    );
    assert_eq!(plan_view.goal, expected_plan["goal"].as_str().unwrap());
    assert_eq!(
        format!("{:?}", plan_view.status),
        expected_plan["status"].as_str().unwrap()
    );
    assert_eq!(
        usize::try_from(plan_view.node_count).unwrap(),
        expected_nodes.len()
    );
    for expected in expected_nodes {
        let id = expected["id"].as_str().unwrap();
        let row = rows
            .iter()
            .find(|row| {
                row.selection
                    .node
                    .is_some_and(|node| node.to_string() == id)
            })
            .expect("stable node row");
        let view = row.node.as_ref().expect("shared node view");
        assert_eq!(view.id.to_string(), id);
        assert_eq!(
            format!("{:?}", view.direct_status),
            expected["status"].as_str().unwrap()
        );
        assert_eq!(
            format!("{:?}", view.effective_status),
            expected["effective_status"].as_str().unwrap()
        );
        assert_eq!(
            view.evidence_count,
            u32::try_from(expected["evidence_count"].as_u64().unwrap()).unwrap()
        );
        assert_eq!(
            view.dependency_blockers.len(),
            expected["dependency_blockers"].as_array().unwrap().len()
        );
        assert_eq!(
            view.completion_gates.satisfied,
            u32::try_from(expected["completion_gates"]["satisfied"].as_u64().unwrap()).unwrap()
        );
        assert_eq!(
            view.summary_id.map(|value| value.to_string()).as_deref(),
            expected["summary_id"].as_str()
        );
        assert_eq!(
            view.last_failure.is_some(),
            expected["last_failure"].is_object()
        );
    }
}

/// Builds the shared conformance session around one restored fixture plan.
///
/// # Parameters
/// - `plan`: Semantic plan restored from the shared protocol fixture.
///
/// # Returns
/// A session with the exact successful prerequisite result represented by the fixture.
fn fixture_session(plan: Plan) -> scriberig_session::Session {
    let root = EntityId::from_bytes([1; 16]);
    let worker_branch = EntityId::from_bytes([13; 16]);
    let result = scriberig_session::BranchResult {
        id: EntityId::from_bytes([14; 16]),
        branch: worker_branch,
        status: scriberig_session::BranchStatus::Completed,
        summary: None,
        result_kind: Some("semantic_changes".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec!["migration-parser-1".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    let mut worker = scriberig_session::Branch::new(worker_branch, "Worker").expect("worker");
    worker.result = Some(result);
    scriberig_session::Session {
        id: EntityId::from_bytes([2; 16]),
        branches: vec![
            scriberig_session::Branch::new(root, "Root").expect("root"),
            worker,
        ],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    }
}

/// Verifies node selection resolves by stable plan/node identity after reorder.
#[test]
fn tree_selection_survives_plan_node_reordering() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let mut plan = sample_plan(plan_id, branch, first, second);
    plan.nodes.reverse();
    let state = TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: EntityId::from_bytes([9; 16]),
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        plan_cursor: Some(PlanSelection {
            plan: plan_id,
            node: Some(first),
        }),
        ..TuiState::default()
    };

    let (tree, _, _) = plan_tree(&state).expect("plan tree");

    assert_eq!(tree.selected_id(), Some(plan_row_id(plan_id, Some(first))));
}

/// Keeps highlighted and detailed plan selections aligned after cursor fallback.
#[test]
fn tree_and_details_share_fallback_plan_selection() {
    let plan_id = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let first = EntityId::from_bytes([3; 16]);
    let second = EntityId::from_bytes([4; 16]);
    let state = TuiState {
        session_snapshot: Some(scriberig_session::Session {
            id: EntityId::from_bytes([9; 16]),
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![sample_plan(plan_id, branch, first, second)],
            execution: scriberig_session::SessionExecutionSelection::default(),
        }),
        cursor: 2,
        ..TuiState::default()
    };

    let (tree, details, _) = plan_tree(&state).expect("plan tree");

    assert_eq!(tree.selected_id(), Some(plan_row_id(plan_id, Some(second))));
    assert!(details.contains(&format!("## Node {second}")));
}
