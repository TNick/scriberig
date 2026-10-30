//! Verifies canonical semantic plan views against the shared wire fixture.

use super::*;

/// Ensures shared node identities and derived plan semantics match the protocol fixture.
#[test]
fn canonical_plan_views_match_shared_cross_surface_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/agent_session_plan_conformance.json"
    ))
    .expect("shared plan fixture");
    let expected_plan = &fixture["session"]["plans"][0];
    let expected_nodes = expected_plan["nodes"].as_array().expect("fixture nodes");
    let (plan, results, node_ids) = fixture_plan();
    let rollups = plan.node_rollups().expect("rollups");

    for (id, expected) in node_ids.into_iter().zip(expected_nodes) {
        let view = plan
            .node_view(id, None, Some(&results), &rollups)
            .expect("node view");
        assert_node_view_matches(&view, expected);
    }
    assert_eq!(
        plan.gates.len(),
        expected_plan["gates"].as_array().unwrap().len()
    );
    assert_eq!(
        plan.evidence.len(),
        expected_plan["evidence"].as_array().unwrap().len()
    );
    assert_eq!(
        plan.summaries.len(),
        expected_plan["summaries"].as_array().unwrap().len()
    );
    assert_eq!(
        plan.ready_nodes_from_results(&results),
        vec![node_ids[1], node_ids[2]]
    );
}

/// Builds canonical plan records and exact results for the shared fixture.
///
/// # Returns
/// The plan, successful prerequisite result, and stable node identities.
///
/// # Errors
/// Panics when the deterministic fixture fails to satisfy a canonical plan invariant.
fn fixture_plan() -> (Plan, [scriberig_session::BranchResult; 1], [EntityId; 3]) {
    let branch = EntityId::from_bytes([1; 16]);
    let parser = EntityId::from_bytes([4; 16]);
    let tests = EntityId::from_bytes([5; 16]);
    let retry = EntityId::from_bytes([6; 16]);
    let gate = EntityId::from_bytes([7; 16]);
    let result = EntityId::from_bytes([14; 16]);
    let validation = EntityId::from_bytes([10; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        EntityId::from_bytes([2; 16]),
        ContinuationAnchor::BranchHead { branch },
        "Implement and validate parser",
    )
    .expect("plan");
    let mut parser_node = node(4, branch, "Implement parser");
    parser_node.status = PlanNodeStatus::Completed;
    parser_node.branch = Some(EntityId::from_bytes([13; 16]));
    parser_node.result = Some(result);
    parser_node.expected_result = Some(ExpectedResult {
        kind: "semantic_changes".to_owned(),
        requires_handle: true,
    });
    let mut test_node = node(5, branch, "Run tests");
    "Verify the parser changes.".clone_into(&mut test_node.purpose);
    test_node.status = PlanNodeStatus::Ready;
    let mut retry_node = node(6, branch, "Retry flaky check");
    retry_node.status = PlanNodeStatus::Ready;
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
    plan.add_node(parser_node).expect("parser node");
    plan.add_node(test_node).expect("tests node");
    plan.add_node(retry_node).expect("retry node");
    plan.add_dependency(parser, tests).expect("dependency");
    plan.dependencies[0].fulfilled_by = Some(result);
    plan.add_gate(PlanGate {
        id: gate,
        owner: tests,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::ValidationPassed {
            profile: Some("parser-contract".to_owned()),
        },
        waiver: None,
    })
    .expect("completion gate");
    attach_fixture_evidence(&mut plan, parser, gate, result, validation);
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
    let results = [scriberig_session::BranchResult {
        id: result,
        branch: EntityId::from_bytes([13; 16]),
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
    }];
    (plan, results, [parser, tests, retry])
}

/// Adds exact branch-result and validation evidence records to one plan.
///
/// # Parameters
/// - `plan`: Canonical plan receiving evidence associations.
/// - `parser`: Node owning successful result evidence.
/// - `gate`: Completion gate owning validation evidence.
/// - `result`: Exact successful branch result.
/// - `validation`: Exact validation entity.
fn attach_fixture_evidence(
    plan: &mut Plan,
    parser: EntityId,
    gate: EntityId,
    result: EntityId,
    validation: EntityId,
) {
    for (id, owner, kind, entity, revision, attributes) in [
        (
            EntityId::from_bytes([8; 16]),
            PlanEvidenceOwner::Node(parser),
            PlanEvidenceKind::BranchResult,
            result,
            None,
            std::collections::BTreeMap::new(),
        ),
        (
            EntityId::from_bytes([9; 16]),
            PlanEvidenceOwner::Gate(gate),
            PlanEvidenceKind::ValidationResult,
            validation,
            Some(scriberig_core::RevisionId::new([16; 32])),
            [
                ("passed".to_owned(), "true".to_owned()),
                ("profile".to_owned(), "parser-contract".to_owned()),
            ]
            .into_iter()
            .collect(),
        ),
    ] {
        plan.attach_evidence(PlanEvidenceAssociation {
            id,
            owner,
            role: PlanEvidenceRole::Supports,
            evidence: PlanEvidenceReference {
                kind,
                reference: PlanSemanticReference {
                    entity,
                    revision,
                    snapshot: None,
                },
                attributes,
            },
            note: None,
            actor: None,
        })
        .expect("evidence");
    }
}

/// Compares one canonical node view with its protocol-shaped fixture value.
///
/// # Parameters
/// - `view`: Canonical application-owned node projection.
/// - `expected`: Protocol-shaped node from the cross-surface fixture.
fn assert_node_view_matches(view: &PlanNodeView, expected: &serde_json::Value) {
    assert_eq!(
        view.direct_status,
        fixture_status(expected["status"].as_str().unwrap())
    );
    assert_eq!(
        view.effective_status,
        fixture_status(expected["effective_status"].as_str().unwrap())
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

/// Maps one fixture lifecycle label to the canonical node status.
///
/// # Parameters
/// - `value`: Serialized lifecycle status.
///
/// # Returns
/// The canonical plan-node status represented by the fixture label.
fn fixture_status(value: &str) -> PlanNodeStatus {
    match value {
        "Planned" => PlanNodeStatus::Planned,
        "Ready" => PlanNodeStatus::Ready,
        "Running" => PlanNodeStatus::Running,
        "Completed" => PlanNodeStatus::Completed,
        "Failed" => PlanNodeStatus::Failed,
        "Abandoned" => PlanNodeStatus::Abandoned,
        _ => panic!("unknown fixture status {value}"),
    }
}
