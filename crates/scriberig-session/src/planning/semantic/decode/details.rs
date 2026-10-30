//! Decodes bounded gate and evidence detail pages for one plan node.

use scriberig_core::EntityId;

use crate::planning::{PlanEvidenceAssociation, PlanEvidenceOwner, PlanGate};

use super::super::{
    PlanSemanticError, PlanSemanticNodeDetails, PlanSemanticRevision, PlanSemanticSnapshot,
};
use super::{
    EVIDENCE_TYPE, EvidencePayload, GATE_TYPE, GatePayload, decode_payload, invalid,
    validate_revision,
};

/// Decodes and pages gate definitions plus evidence associated with their owner.
///
/// # Parameters
/// - `snapshot`: Exact semantic records selected by the repository.
/// - `node`: Stable node identity whose details are requested.
/// - `offset`: Number of matching records to skip, per category.
/// - `limit`: Maximum records returned, per category.
///
/// # Returns
/// The requested gate and evidence pages.
///
/// # Errors
/// Returns an error for malformed or mismatched payloads.
pub(super) fn node_details(
    snapshot: &PlanSemanticSnapshot,
    node: EntityId,
    offset: usize,
    limit: usize,
) -> Result<PlanSemanticNodeDetails, PlanSemanticError> {
    let mut gates = Vec::new();
    let mut evidence = Vec::new();
    for object in &snapshot.objects {
        if object.object_type.as_str() == GATE_TYPE {
            let gate = decode_gate(object, snapshot.plan)?;
            if gate.owner == node {
                gates.push(gate);
            }
        } else if object.object_type.as_str() == EVIDENCE_TYPE {
            evidence.push(decode_evidence(object, snapshot.plan)?);
        }
    }
    gates.sort_by_key(|gate| gate.id);
    let gate_ids = gates.iter().map(|gate| gate.id).collect::<Vec<_>>();
    let evidence = evidence
        .into_iter()
        .filter(|item| match item.owner {
            PlanEvidenceOwner::Node(owner) => owner == node,
            PlanEvidenceOwner::Gate(gate) => gate_ids.contains(&gate),
        })
        .collect::<Vec<_>>();
    let gates = page(gates, offset, limit);
    let evidence = page(evidence, offset, limit);
    Ok(PlanSemanticNodeDetails { gates, evidence })
}

/// Decodes and validates one gate record.
///
/// # Parameters
/// - `object`: Semantic object to decode.
/// - `plan`: Expected owning plan identity.
///
/// # Returns
/// The typed gate when its payload is valid.
///
/// # Errors
/// Returns an error when its revision or ownership is invalid.
fn decode_gate(
    object: &PlanSemanticRevision,
    plan: EntityId,
) -> Result<PlanGate, PlanSemanticError> {
    validate_revision(object)?;
    let payload: GatePayload = decode_payload(object)?;
    if payload.schema_version != 1 || payload.plan != plan {
        return Err(invalid("mismatched or unsupported gate"));
    }
    Ok(PlanGate {
        id: object.entity,
        owner: payload.owner,
        phase: payload.phase,
        requirement: payload.requirement,
        waiver: payload.waiver,
    })
}

/// Decodes and validates one evidence association record.
///
/// # Parameters
/// - `object`: Semantic object to decode.
/// - `plan`: Expected owning plan identity.
///
/// # Returns
/// The typed evidence association when its payload is valid.
///
/// # Errors
/// Returns an error when its revision or ownership is invalid.
fn decode_evidence(
    object: &PlanSemanticRevision,
    plan: EntityId,
) -> Result<PlanEvidenceAssociation, PlanSemanticError> {
    validate_revision(object)?;
    let payload: EvidencePayload = decode_payload(object)?;
    if payload.schema_version != 1 || payload.plan != plan {
        return Err(invalid("mismatched or unsupported evidence"));
    }
    Ok(PlanEvidenceAssociation {
        id: object.entity,
        owner: payload.owner,
        role: payload.role,
        evidence: payload.evidence,
        note: payload.note,
        actor: payload.actor,
    })
}

/// Returns one stable bounded page.
///
/// # Parameters
/// - `records`: Records already selected and sorted by stable identity.
/// - `offset`: Number of records to skip.
/// - `limit`: Maximum number of records to return.
///
/// # Returns
/// At most `limit` records after skipping `offset` records.
fn page<T>(records: Vec<T>, offset: usize, limit: usize) -> Vec<T> {
    if limit == 0 {
        return Vec::new();
    }
    records.into_iter().skip(offset).take(limit).collect()
}
