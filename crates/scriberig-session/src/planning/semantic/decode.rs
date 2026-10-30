//! Reconstructs plan projections from independently loaded semantic objects.

#[path = "decode/assemble.rs"]
mod assemble;
#[path = "decode/node_refs.rs"]
mod node_refs;

pub(super) use assemble::into_plan;

use scriberig_core::{EncodingVersion, EntityId, revision_id};
use serde::Deserialize;
use std::collections::BTreeMap;

use super::super::{
    CoordinatorClaim, ExpectedResult, ModelAssignment, PlanEvidenceOwner, PlanEvidenceReference,
    PlanEvidenceRole, PlanGatePhase, PlanGateRequirement, PlanGateWaiver, PlanNodeParent,
    PlanNodeStatus, PlanStatus, WorkerClaim,
};
use super::encode::{
    DEPENDENCY_TYPE, DependencyIdentity, EVIDENCE_TYPE, GATE_TYPE, HISTORY_TYPE, MEMBERSHIP_TYPE,
    NODE_TYPE, PLAN_TYPE, STRUCTURE_TYPE, SUMMARY_LINK_TYPE, SUMMARY_TYPE, parent_identity,
    relation_id,
};
use super::{
    PlanSemanticError, PlanSemanticHistory, PlanSemanticNodeRef, PlanSemanticRevision,
    PlanSemanticSnapshot,
};

/// Payload for the semantic plan root object.
#[derive(Deserialize)]
struct PlanPayload {
    schema_version: u16,
    session: Option<EntityId>,
    anchor: Option<crate::ContinuationAnchor>,
    goal: String,
    mutation_revision: u64,
    status: PlanStatus,
    root_branch: EntityId,
    coordinator: Option<EntityId>,
    coordinator_cursor: Option<EntityId>,
    coordinator_claim: Option<CoordinatorClaim>,
    coordinator_generation: u64,
}

/// Payload for one semantic plan node object.
#[derive(Deserialize)]
struct NodePayload {
    schema_version: u16,
    plan: EntityId,
    title: String,
    purpose: String,
    expected_result: Option<ExpectedResult>,
    semantic_scope: Vec<String>,
    status: PlanNodeStatus,
    branch: Option<EntityId>,
    result: Option<EntityId>,
    superseded_by: Option<EntityId>,
    #[serde(default)]
    abandon_reason: Option<super::super::PlanAbandonReason>,
    #[serde(default)]
    last_failure: Option<PlanEvidenceReference>,
    assignment: Option<ModelAssignment>,
    claim: Option<WorkerClaim>,
    claim_generation: u64,
}

/// Payload linking a node to its plan in stable order.
#[derive(Deserialize)]
struct MembershipPayload {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    ordinal: u64,
}

/// Payload linking a node to its recursive branch parent.
#[derive(Deserialize)]
struct StructurePayload {
    schema_version: u16,
    plan: EntityId,
    root_branch: EntityId,
    child: EntityId,
    parent: Option<PlanNodeParent>,
    ordinal: u64,
}

/// Payload describing a dependency edge between plan nodes.
#[derive(Deserialize)]
struct DependencyPayload {
    schema_version: u16,
    plan: EntityId,
    prerequisite: EntityId,
    dependent: EntityId,
    fulfilled_by: Option<EntityId>,
}

/// Payload recording one semantic plan mutation revision.
#[derive(Deserialize)]
struct HistoryPayload {
    schema_version: u16,
    plan: EntityId,
    mutation_revision: u64,
    reason: Option<String>,
}

/// Stable key used to derive a semantic history relation identity.
#[derive(serde::Serialize)]
struct HistoryIdentity {
    plan: EntityId,
    mutation_revision: u64,
}

/// Payload containing a durable summary for a plan node.
#[derive(Deserialize)]
struct SummaryPayload {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    text: String,
    sources: Vec<super::super::PlanSemanticReference>,
}

/// Payload connecting a summary object to its plan node.
#[derive(Deserialize)]
struct SummaryLinkPayload {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    summary: EntityId,
}

/// Payload describing a plan gate requirement or waiver.
#[derive(Deserialize)]
struct GatePayload {
    schema_version: u16,
    plan: EntityId,
    owner: EntityId,
    phase: PlanGatePhase,
    requirement: PlanGateRequirement,
    waiver: Option<PlanGateWaiver>,
}

/// Payload linking evidence to a plan or node owner.
#[derive(Deserialize)]
struct EvidencePayload {
    schema_version: u16,
    plan: EntityId,
    owner: PlanEvidenceOwner,
    role: PlanEvidenceRole,
    evidence: PlanEvidenceReference,
    note: Option<String>,
    actor: Option<EntityId>,
}

/// Queries relation-backed node positions without decoding Plan payload.
///
/// - `snapshot`: Exact semantic plan object set.
///
/// Node identities, revisions, and direct parents in stable order.
///
/// Returns an error for malformed relations.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn node_refs(
    snapshot: &PlanSemanticSnapshot,
) -> Result<Vec<PlanSemanticNodeRef>, PlanSemanticError> {
    node_refs::node_refs(snapshot)
}

/// Decodes safe mutation metadata from a semantic plan object set.
///
/// - `snapshot`: Semantic plan records to inspect.
///
/// Ordered revision metadata.
///
/// Returns an error for malformed or mismatched history objects.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn history_metadata(
    snapshot: &PlanSemanticSnapshot,
) -> Result<Vec<PlanSemanticHistory>, PlanSemanticError> {
    let mut metadata = BTreeMap::new();
    for object in &snapshot.objects {
        if object.object_type.as_str() != HISTORY_TYPE {
            continue;
        }
        validate_revision(object)?;
        let payload: HistoryPayload = decode_payload(object)?;
        if payload.schema_version != 1
            || payload.plan != snapshot.plan
            || relation_id(
                HISTORY_TYPE,
                &HistoryIdentity {
                    plan: payload.plan,
                    mutation_revision: payload.mutation_revision,
                },
            )? != object.entity
            || metadata
                .insert(payload.mutation_revision, payload.reason)
                .is_some()
        {
            return Err(invalid(
                "duplicate, mismatched, or unsupported history record",
            ));
        }
    }
    Ok(metadata
        .into_iter()
        .map(|(mutation_revision, reason)| PlanSemanticHistory {
            mutation_revision,
            reason,
        })
        .collect())
}

/// Verifies the canonical content identity before decoding an object.
///
/// - `object`: Semantic object to verify.
///
/// Unit when the payload hashes to its recorded immutable revision.
///
/// Returns `InvalidObject` when the revision does not match its payload.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn validate_revision(object: &PlanSemanticRevision) -> Result<(), PlanSemanticError> {
    let expected = revision_id(EncodingVersion::V1, &object.object_type, &object.payload)
        .map_err(|error| PlanSemanticError::Serialization(format!("{error:?}")))?;
    if expected != object.revision {
        return Err(invalid("object revision does not match canonical payload"));
    }
    Ok(())
}

/// Decodes one typed JSON payload from an immutable object.
///
/// - `object`: Revision whose payload is decoded.
///
/// The versioned typed payload.
///
/// Returns a serialization error when bytes are invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn decode_payload<T: for<'de> Deserialize<'de>>(
    object: &PlanSemanticRevision,
) -> Result<T, PlanSemanticError> {
    serde_json::from_slice(&object.payload)
        .map_err(|error| PlanSemanticError::Serialization(error.to_string()))
}

/// Creates a typed malformed-object diagnostic.
///
/// - `message`: Bounded explanation of the malformed object.
///
/// A semantic plan decoding error.
/// # Returns
/// The operation result.
fn invalid(message: impl Into<String>) -> PlanSemanticError {
    PlanSemanticError::InvalidObject(message.into())
}
