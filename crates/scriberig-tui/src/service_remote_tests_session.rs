//! Verifies remote session reads preserve durable plan semantics.

use super::*;

/// Preserves plans and explicit dependency results from a remote session read.
/// The fixture also exercises gate, evidence, summary, and root-order mapping.
#[test]
#[allow(clippy::too_many_lines)]
fn remote_session_mapping_preserves_plan_results_and_assignments() {
    let root = EntityId::from_bytes([51; 16]);
    let prerequisite = EntityId::from_bytes([52; 16]);
    let dependent = EntityId::from_bytes([53; 16]);
    let result = EntityId::from_bytes([54; 16]);
    let response = SessionReadResponse {
        session: SessionReference {
            id: EntityId::from_bytes([50; 16]).to_string(),
        },
        branches: Vec::new(),
        plans: vec![scriberig_protocol::SessionPlan {
            id: EntityId::from_bytes([55; 16]).to_string(),
            revision: 2,
            goal: "test plan".to_owned(),
            status: scriberig_protocol::SessionPlanStatus::Running,
            anchor: Some(scriberig_protocol::SessionContinuationAnchor::BranchHead {
                branch: root.to_string(),
            }),
            root_branch: root.to_string(),
            nodes: vec![scriberig_protocol::SessionPlanNode {
                id: prerequisite.to_string(),
                parent: root.to_string(),
                title: "producer".to_owned(),
                purpose: "produce a semantic result".to_owned(),
                status: scriberig_protocol::SessionPlanNodeStatus::Completed,
                exact_revision: None,
                effective_status: None,
                contains_ignored: false,
                contains_abandoned: false,
                abandon_reason: None,
                branch: None,
                result: Some(result.to_string()),
                superseded_by: None,
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: Some(scriberig_protocol::SessionModelAssignment {
                    provider_id: "provider-a".to_owned(),
                    model_id: "model-a".to_owned(),
                    thinking_level: Some("careful".to_owned()),
                }),
                claim: None,
                summary_id: None,
                summary_preview: None,
                dependency_blockers: Vec::new(),
                entry_gates: scriberig_protocol::SessionPlanGateCounts::default(),
                completion_gates: scriberig_protocol::SessionPlanGateCounts::default(),
                evidence_count: 0,
                can_retry: false,
                last_failure: None,
            }],
            dependencies: vec![scriberig_protocol::SessionPlanDependency {
                prerequisite: prerequisite.to_string(),
                dependent: dependent.to_string(),
                fulfilled_by: Some(result.to_string()),
            }],
            coordinator: Some(root.to_string()),
            coordinator_cursor: Some(dependent.to_string()),
            coordinator_claim: None,
            history: Vec::new(),
            exact_revision: None,
            snapshot: None,
            root_nodes: vec![dependent.to_string()],
            status_counts: scriberig_protocol::SessionPlanStatusCounts::default(),
            gates: vec![scriberig_protocol::SessionPlanGate {
                id: EntityId::from_bytes([56; 16]).to_string(),
                owner: dependent.to_string(),
                phase: scriberig_protocol::SessionPlanGatePhase::Completion,
                requirement: scriberig_protocol::SessionPlanGateRequirement::EvidencePresent {
                    kind: scriberig_protocol::SessionPlanEvidenceKind::Artifact,
                },
                waiver: None,
            }],
            evidence: vec![scriberig_protocol::SessionPlanEvidenceAssociation {
                id: EntityId::from_bytes([57; 16]).to_string(),
                owner: scriberig_protocol::SessionPlanEvidenceOwner::Gate(
                    EntityId::from_bytes([56; 16]).to_string(),
                ),
                role: scriberig_protocol::SessionPlanEvidenceRole::Supports,
                evidence: scriberig_protocol::SessionPlanEvidenceReference {
                    kind: scriberig_protocol::SessionPlanEvidenceKind::Artifact,
                    reference: scriberig_protocol::SessionSemanticReference {
                        entity: EntityId::from_bytes([58; 16]).to_string(),
                        revision: None,
                        snapshot: None,
                    },
                    attributes: std::collections::BTreeMap::new(),
                },
                note: None,
                actor: None,
            }],
            summaries: vec![scriberig_protocol::SessionPlanSummary {
                id: EntityId::from_bytes([59; 16]).to_string(),
                node: dependent.to_string(),
                text: "remote summary".to_owned(),
                sources: Vec::new(),
            }],
        }],
        execution: scriberig_protocol::SessionExecutionSelection::default(),
    };

    let session = session_from_response(response).expect("mapped session");
    assert_eq!(session.plans[0].dependencies[0].fulfilled_by, Some(result));
    assert_eq!(session.plans[0].coordinator, Some(root));
    assert_eq!(session.plans[0].revision, 2);
    assert_eq!(session.plans[0].goal, "test plan");
    assert_eq!(session.plans[0].root_nodes, vec![dependent]);
    assert_eq!(session.plans[0].gates.len(), 1);
    assert_eq!(session.plans[0].evidence.len(), 1);
    assert_eq!(session.plans[0].summaries[0].text, "remote summary");
    assert_eq!(
        session.plans[0].status,
        scriberig_session::PlanStatus::Running
    );
    assert_eq!(
        session.plans[0].nodes[0]
            .assignment
            .as_ref()
            .unwrap()
            .thinking_level
            .as_deref(),
        Some("careful")
    );
}
