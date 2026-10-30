//! Verifies the server preserves safe durable plan projections.

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, ModelAssignment, Plan, PlanDependency, PlanEvidenceAssociation, PlanEvidenceKind,
    PlanEvidenceOwner, PlanEvidenceReference, PlanEvidenceRole, PlanGate, PlanGatePhase,
    PlanGateRequirement, PlanNode, PlanNodeStatus, PlanNodeSummary, PlanSemanticReference, Session,
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
            source_snapshot: None,
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
                    abandon_reason: None,
                    last_failure: None,
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
                    abandon_reason: None,
                    last_failure: None,
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
            summaries: Vec::new(),
            evidence: Vec::new(),
            gates: Vec::new(),
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

/// Verifies server projection matches rich semantics in the shared wire fixture.
#[test]
fn session_response_plan_semantics_match_cross_surface_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/agent_session_plan_conformance.json"
    ))
    .expect("shared fixture");
    let expected: scriberig_protocol::SessionReadResponse =
        serde_json::from_value(fixture["session"].clone()).expect("wire fixture");
    let expected_plan = &expected.plans[0];
    let root = EntityId::from_bytes([1; 16]);
    let session_id = EntityId::from_bytes([2; 16]);
    let result = EntityId::from_bytes([14; 16]);
    let branch = EntityId::from_bytes([13; 16]);
    let mut session = Session {
        id: session_id,
        branches: vec![Branch::new(root, "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![semantic_fixture_plan(session_id, root)],
        execution: SessionExecutionSelection::default(),
    };
    let mut root_branch = Branch::new(root, "root").expect("root");
    let mut worker_branch = Branch::new(branch, "parser").expect("worker");
    worker_branch.parent = Some(root);
    worker_branch.result = Some(scriberig_session::BranchResult {
        id: result,
        branch,
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
    });
    root_branch.children.push(branch);
    session.branches = vec![root_branch, worker_branch];
    let plan = &mut session.plans[0];
    plan.dependencies[0].fulfilled_by = Some(result);
    plan.nodes[0].result = Some(result);
    plan.nodes[0].branch = Some(branch);
    let actual = super::session_response(session).expect("server projection");
    let actual_plan = &actual.plans[0];

    assert_eq!(actual_plan.nodes.len(), expected_plan.nodes.len());
    assert_eq!(actual_plan.id, expected_plan.id);
    assert_eq!(actual_plan.gates, expected_plan.gates);
    assert_eq!(actual_plan.evidence, expected_plan.evidence);
    assert_eq!(actual_plan.summaries, expected_plan.summaries);
    for (actual_node, expected_node) in actual_plan.nodes.iter().zip(&expected_plan.nodes) {
        assert_eq!(actual_node.id, expected_node.id);
        assert_eq!(actual_node.status, expected_node.status);
        assert_eq!(actual_node.effective_status, expected_node.effective_status);
        assert_eq!(actual_node.entry_gates, expected_node.entry_gates);
        assert_eq!(actual_node.completion_gates, expected_node.completion_gates);
        assert_eq!(actual_node.evidence_count, expected_node.evidence_count);
        assert_eq!(
            actual_node.dependency_blockers,
            expected_node.dependency_blockers
        );
        assert_eq!(actual_node.last_failure, expected_node.last_failure);
    }
    assert_eq!(actual_plan.dependencies, expected_plan.dependencies);
    assert_eq!(
        actual_plan
            .nodes
            .iter()
            .filter(|node| node.status == scriberig_protocol::SessionPlanNodeStatus::Ready)
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>(),
        expected_plan
            .nodes
            .iter()
            .filter(|node| node.status == scriberig_protocol::SessionPlanNodeStatus::Ready)
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>()
    );
}

/// Builds canonical server fixture values from the shared plan projection.
///
/// # Parameters
/// - `session`: Stable owner session identity.
/// - `root`: Existing root branch identity.
///
/// # Returns
/// A plan with stable nodes, dependency, gate, evidence, and summary records.
fn semantic_fixture_plan(session: EntityId, root: EntityId) -> Plan {
    let parser = EntityId::from_bytes([4; 16]);
    let tests = EntityId::from_bytes([5; 16]);
    let retry = EntityId::from_bytes([6; 16]);
    let gate = EntityId::from_bytes([7; 16]);
    let branch = EntityId::from_bytes([13; 16]);
    let result = EntityId::from_bytes([14; 16]);
    let validation = EntityId::from_bytes([10; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        session,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root },
        "Implement and validate parser",
    )
    .expect("plan");
    let mut source = semantic_node(parser, root, "Implement parser", PlanNodeStatus::Completed);
    source.branch = Some(branch);
    source.result = Some(result);
    source.expected_result = Some(scriberig_session::ExpectedResult {
        kind: "semantic_changes".to_owned(),
        requires_handle: true,
    });
    plan.add_node(source).expect("source node");
    plan.add_node(semantic_node(
        tests,
        root,
        "Run tests",
        PlanNodeStatus::Ready,
    ))
    .expect("tests node");
    let mut retry_node = semantic_node(retry, root, "Retry flaky check", PlanNodeStatus::Ready);
    retry_node.last_failure = Some(PlanEvidenceReference {
        kind: PlanEvidenceKind::Failure,
        reference: PlanSemanticReference {
            entity: retry,
            revision: Some(scriberig_core::RevisionId::new([15; 32])),
            snapshot: None,
        },
        attributes: [("category".to_owned(), "transient".to_owned())]
            .into_iter()
            .collect(),
    });
    plan.add_node(retry_node).expect("retry node");
    plan.add_dependency(parser, tests).expect("dependency");
    plan.add_gate(PlanGate {
        id: gate,
        owner: tests,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::ValidationPassed {
            profile: Some("parser-contract".to_owned()),
        },
        waiver: None,
    })
    .expect("gate");
    attach_server_fixture_evidence(&mut plan, parser, gate, result, validation);
    plan.update_summary(PlanNodeSummary {
        id: EntityId::from_bytes([12; 16]),
        node: tests,
        text: "Parser checks passed.".to_owned(),
        sources: vec![PlanSemanticReference {
            entity: validation,
            revision: Some(scriberig_core::RevisionId::new([16; 32])),
            snapshot: None,
        }],
    })
    .expect("summary");
    plan
}

/// Attaches exact branch-result and validation evidence to the server fixture.
///
/// # Parameters
/// - `plan`: Canonical fixture plan receiving evidence.
/// - `parser`: Node receiving branch-result evidence.
/// - `gate`: Gate receiving validation evidence.
/// - `result`: Exact completed branch result identity.
/// - `validation`: Exact validation entity identity.
fn attach_server_fixture_evidence(
    plan: &mut Plan,
    parser: EntityId,
    gate: EntityId,
    result: EntityId,
    validation: EntityId,
) {
    let branch_result = PlanEvidenceAssociation {
        id: EntityId::from_bytes([8; 16]),
        owner: PlanEvidenceOwner::Node(parser),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::BranchResult,
            reference: PlanSemanticReference {
                entity: result,
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: None,
        actor: None,
    };
    let validation_evidence = PlanEvidenceAssociation {
        id: EntityId::from_bytes([9; 16]),
        owner: PlanEvidenceOwner::Gate(gate),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::ValidationResult,
            reference: PlanSemanticReference {
                entity: validation,
                revision: Some(scriberig_core::RevisionId::new([16; 32])),
                snapshot: None,
            },
            attributes: [
                ("passed".to_owned(), "true".to_owned()),
                ("profile".to_owned(), "parser-contract".to_owned()),
            ]
            .into_iter()
            .collect(),
        },
        note: None,
        actor: None,
    };
    plan.attach_evidence(branch_result)
        .expect("branch result evidence");
    plan.attach_evidence(validation_evidence)
        .expect("validation evidence");
}

/// Creates one stable server conformance node.
///
/// # Parameters
/// - `id`: Stable plan node identity.
/// - `root`: Structural parent branch identity.
/// - `title`: Human-readable node title and purpose.
/// - `status`: Direct lifecycle status.
///
/// # Returns
/// A plan node without worker or result bindings.
fn semantic_node(id: EntityId, root: EntityId, title: &str, status: PlanNodeStatus) -> PlanNode {
    PlanNode {
        id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(root)),
        title: title.to_owned(),
        purpose: title.to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status,
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
