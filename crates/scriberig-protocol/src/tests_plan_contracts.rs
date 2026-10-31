//! Verifies the durable plan protocol contracts.

use super::*;

#[allow(
    clippy::too_many_lines,
    reason = "the test exercises the full plan protocol contract table"
)]
/// Verifies plan mutation requests preserve each operation over JSON.
pub(super) fn session_plan_contracts_round_trip() {
    session_snapshot_contracts_round_trip();
    let session = SessionReference {
        id: "session-1".to_owned(),
    };
    round_trip(&SessionPlanCreateRequest {
        session: session.clone(),
        anchor: SessionContinuationAnchor::BranchHead {
            branch: "branch-1".to_owned(),
        },
        goal: "create".to_owned(),
        nodes: vec![SessionPlanNodeDraft {
            key: "node-a".to_owned(),
            parent_index: None,
            title: "Node A".to_owned(),
            purpose: "perform work".to_owned(),
            expected_result: None,
            semantic_scope: vec!["entity:a".to_owned()],
            assignment: None,
        }],
        dependencies: Vec::new(),
    });
    round_trip(&SessionPlanLifecycleRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 4,
        action: SessionPlanAction::Pause,
    });
    round_trip(&SessionPlanMutationRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 5,
        mutation: SessionPlanMutation::AddNextStep {
            node: node_draft("next"),
        },
    });
    round_trip(&SessionPlanMutationRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 6,
        mutation: SessionPlanMutation::Decompose {
            parent: "node-a".to_owned(),
            children: vec![node_draft("child")],
            required: true,
        },
    });
    round_trip(&SessionPlanMutationRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 7,
        mutation: SessionPlanMutation::AddGate {
            owner: "node-a".to_owned(),
            phase: SessionPlanGatePhase::Completion,
            requirement: SessionPlanGateRequirement::EvidencePresent {
                kind: SessionPlanEvidenceKind::ValidationResult,
            },
        },
    });
    round_trip(&SessionPlanMutationRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 8,
        mutation: SessionPlanMutation::AttachEvidence {
            owner: SessionPlanEvidenceOwner::Node("node-a".to_owned()),
            role: SessionPlanEvidenceRole::Supports,
            evidence: SessionPlanEvidenceReference {
                kind: SessionPlanEvidenceKind::ValidationResult,
                reference: SessionSemanticReference {
                    entity: "validation-1".to_owned(),
                    revision: Some("revision-1".to_owned()),
                    snapshot: Some("snapshot-1".to_owned()),
                },
                attributes: std::collections::BTreeMap::new(),
            },
            note: Some("focused validation".to_owned()),
        },
    });
    round_trip(&SessionPlanMutationRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 9,
        mutation: SessionPlanMutation::UpdateSummary {
            node: "node-a".to_owned(),
            text: "Implementation is complete.".to_owned(),
            sources: vec![SessionSemanticReference {
                entity: "validation-1".to_owned(),
                revision: Some("revision-1".to_owned()),
                snapshot: Some("snapshot-1".to_owned()),
            }],
        },
    });
    round_trip(&SessionPlanMutationRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 10,
        mutation: SessionPlanMutation::WaiveGate {
            gate: "gate-1".to_owned(),
            reason: "approved by owner".to_owned(),
        },
    });
    round_trip(&SessionPlanMutationRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 11,
        mutation: SessionPlanMutation::RetryFailedNode {
            node: "node-a".to_owned(),
        },
    });
    round_trip(&SessionPlanMutationRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        expected_revision: 12,
        mutation: SessionPlanMutation::RecoverFailedPlan,
    });
    round_trip(&SessionPlanAssignmentRequest {
        session: session.clone(),
        plan: "plan-1".to_owned(),
        node: "node-a".to_owned(),
        expected_revision: 13,
        assignment: Some(SessionModelAssignment {
            provider_id: "provider-safe".to_owned(),
            model_id: "model-safe".to_owned(),
            thinking_level: None,
        }),
    });
    round_trip(&SessionPlanReplanRequest {
        session,
        plan: "plan-1".to_owned(),
        expected_revision: 14,
        additions: Vec::new(),
        dependencies: vec![(
            SessionPlanNodeReference::Existing {
                id: "node-a".to_owned(),
            },
            SessionPlanNodeReference::Proposed {
                key: "node-b".to_owned(),
            },
        )],
        removed_dependencies: Vec::new(),
        superseded: Vec::new(),
        assignment_changes: Vec::new(),
        preserved_terminal_nodes: Vec::new(),
        reason: "new information".to_owned(),
    });
}

/// Builds a provider-neutral plan node proposal for protocol round trips.
///
/// - `key`: Request-local identity for the proposal.
///
/// A valid minimal node draft.
///
/// # Returns
/// The constructed proposal draft.
fn node_draft(key: &str) -> SessionPlanNodeDraft {
    SessionPlanNodeDraft {
        key: key.to_owned(),
        parent_index: None,
        title: "Plan node".to_owned(),
        purpose: "Perform bounded work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        assignment: None,
    }
}
