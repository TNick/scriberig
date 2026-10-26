//! Shared durable fixtures for plan write and read scope measurements.

use std::collections::BTreeMap;

use scriberig_core::EntityId;
use scriberig_session::{
    PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceReference,
    PlanEvidenceRole, PlanNode, PlanNodeParent, PlanNodeStatus, PlanSemanticReference,
};

/// External semantic object referenced by every fixture evidence association.
const EVIDENCE_SUBJECT: EntityId = EntityId::from_bytes([220; 16]);

/// Builds one planned semantic node with no assignment or execution attempt.
///
/// # Parameters
/// - `id`: Stable node identity.
/// - `parent_branch`: Root branch that structurally parents the node.
/// - `title`: Human-readable node title.
///
/// # Returns
/// A planned node parented by the plan's root branch.
pub(super) fn planned_node(id: EntityId, parent_branch: EntityId, title: &str) -> PlanNode {
    PlanNode {
        id,
        parent: Some(PlanNodeParent::Branch(parent_branch)),
        title: title.to_owned(),
        purpose: title.to_owned(),
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
    }
}

/// Attaches one node-owned or gate-owned evidence association.
///
/// # Parameters
/// - `id`: Stable association identity.
/// - `owner`: Node or gate the evidence supports.
/// - `role`: Evidence role relative to its owner.
///
/// # Returns
/// A durable evidence association referencing an external semantic object.
pub(super) fn association(
    id: EntityId,
    owner: PlanEvidenceOwner,
    role: PlanEvidenceRole,
) -> PlanEvidenceAssociation {
    PlanEvidenceAssociation {
        id,
        owner,
        role,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::Artifact,
            reference: PlanSemanticReference {
                entity: EVIDENCE_SUBJECT,
                revision: None,
                snapshot: None,
            },
            attributes: BTreeMap::new(),
        },
        note: None,
        actor: None,
    }
}
