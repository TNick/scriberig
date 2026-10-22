//! Reconstructs plan projections from independently loaded semantic objects.

#[path = "decode/node_refs.rs"]
mod node_refs;

use scriberig_core::{EncodingVersion, EntityId, revision_id};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    CoordinatorClaim, ExpectedResult, ModelAssignment, Plan, PlanDependency, PlanNode,
    PlanNodeParent, PlanNodeStatus, PlanRevision, PlanStatus, WorkerClaim,
};
use super::encode::{
    DEPENDENCY_TYPE, DependencyIdentity, HISTORY_TYPE, MEMBERSHIP_TYPE, NODE_TYPE, PLAN_TYPE,
    RelationIdentity, STRUCTURE_TYPE, parent_identity, relation_id,
};
use super::{
    PlanSemanticError, PlanSemanticHistory, PlanSemanticNodeRef, PlanSemanticRevision,
    PlanSemanticSnapshot,
};

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
    assignment: Option<ModelAssignment>,
    claim: Option<WorkerClaim>,
    claim_generation: u64,
}

#[derive(Deserialize)]
struct MembershipPayload {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    ordinal: u64,
}

#[derive(Deserialize)]
struct StructurePayload {
    schema_version: u16,
    plan: EntityId,
    root_branch: EntityId,
    child: EntityId,
    parent: Option<PlanNodeParent>,
    ordinal: u64,
}

#[derive(Deserialize)]
struct DependencyPayload {
    schema_version: u16,
    plan: EntityId,
    prerequisite: EntityId,
    dependent: EntityId,
    fulfilled_by: Option<EntityId>,
}

#[derive(Deserialize)]
struct HistoryPayload {
    schema_version: u16,
    plan: EntityId,
    mutation_revision: u64,
    reason: Option<String>,
}

#[derive(serde::Serialize)]
struct HistoryIdentity {
    plan: EntityId,
    mutation_revision: u64,
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

/// Assembles one typed plan projection after validating every object revision.
///
/// - `snapshot`: Current immutable plan object set.
/// - `history`: Historical compatibility records retained by the caller.
///
/// The assembled plan projection.
///
/// Returns an error for invalid, missing, duplicate, or unsupported records.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn into_plan(
    snapshot: &PlanSemanticSnapshot,
    history: Vec<PlanRevision>,
) -> Result<Plan, PlanSemanticError> {
    let mut plan_payload = None;
    let mut nodes = BTreeMap::new();
    let mut memberships = BTreeMap::new();
    let mut structures = BTreeMap::new();
    let mut dependencies = Vec::new();
    let mut history_metadata = BTreeMap::new();

    for object in &snapshot.objects {
        validate_revision(object)?;
        match object.object_type.as_str() {
            PLAN_TYPE => {
                if object.entity != snapshot.plan || plan_payload.is_some() {
                    return Err(invalid("duplicate or mismatched plan object"));
                }
                let payload: PlanPayload = decode_payload(object)?;
                if payload.schema_version != 1 {
                    return Err(invalid("unsupported plan payload version"));
                }
                plan_payload = Some(payload);
            }
            NODE_TYPE => {
                let payload: NodePayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || nodes.contains_key(&object.entity)
                {
                    return Err(invalid("duplicate, mismatched, or unsupported node object"));
                }
                nodes.insert(object.entity, payload);
            }
            MEMBERSHIP_TYPE => {
                let payload: MembershipPayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || relation_id(
                        MEMBERSHIP_TYPE,
                        &RelationIdentity {
                            owner: payload.plan,
                            target: payload.node,
                        },
                    )? != object.entity
                    || memberships.insert(payload.node, payload.ordinal).is_some()
                {
                    return Err(invalid("duplicate, mismatched, or unsupported membership"));
                }
            }
            STRUCTURE_TYPE => {
                let payload: StructurePayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || relation_id(
                        STRUCTURE_TYPE,
                        &RelationIdentity {
                            owner: payload.child,
                            target: payload.parent.map_or(payload.root_branch, parent_identity),
                        },
                    )? != object.entity
                    || structures
                        .insert(payload.child, (payload.ordinal, payload.parent))
                        .is_some()
                {
                    return Err(invalid("duplicate or unsupported structure relation"));
                }
            }
            DEPENDENCY_TYPE => {
                let payload: DependencyPayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || relation_id(
                        DEPENDENCY_TYPE,
                        &DependencyIdentity {
                            plan: payload.plan,
                            prerequisite: payload.prerequisite,
                            dependent: payload.dependent,
                        },
                    )? != object.entity
                {
                    return Err(invalid("mismatched or unsupported dependency"));
                }
                dependencies.push(PlanDependency {
                    prerequisite: payload.prerequisite,
                    dependent: payload.dependent,
                    fulfilled_by: payload.fulfilled_by,
                });
            }
            HISTORY_TYPE => {
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
                    || history_metadata
                        .insert(payload.mutation_revision, payload.reason)
                        .is_some()
                {
                    return Err(invalid(
                        "duplicate, mismatched, or unsupported history record",
                    ));
                }
            }
            type_name => {
                return Err(invalid(format!(
                    "unsupported semantic object type {type_name}"
                )));
            }
        }
    }

    let payload = plan_payload.ok_or_else(|| invalid("missing plan object"))?;
    if nodes.len() != memberships.len() || nodes.len() != structures.len() {
        return Err(invalid("node, membership, and structure sets differ"));
    }
    let mut ordered_nodes = memberships.into_iter().collect::<Vec<_>>();
    ordered_nodes.sort_by_key(|(node, ordinal)| (*ordinal, *node));
    let mut plan_nodes = Vec::with_capacity(ordered_nodes.len());
    let mut root_nodes = Vec::new();
    let mut ordinals = BTreeSet::new();
    for (node_id, ordinal) in ordered_nodes {
        if !ordinals.insert(ordinal) {
            return Err(invalid("duplicate plan membership ordinal"));
        }
        let node = nodes
            .remove(&node_id)
            .ok_or_else(|| invalid("membership points to a missing node"))?;
        let (structure_ordinal, parent) = structures
            .remove(&node_id)
            .ok_or_else(|| invalid("node has no structural relation"))?;
        if structure_ordinal != ordinal {
            return Err(invalid("membership and structure order disagree"));
        }
        if matches!(parent, Some(PlanNodeParent::Branch(_)) | None) {
            root_nodes.push(node_id);
        }
        plan_nodes.push(PlanNode {
            id: node_id,
            parent,
            title: node.title,
            purpose: node.purpose,
            expected_result: node.expected_result,
            semantic_scope: node.semantic_scope,
            status: node.status,
            branch: node.branch,
            result: node.result,
            superseded_by: node.superseded_by,
            assignment: node.assignment,
            claim: node.claim,
            claim_generation: node.claim_generation,
        });
    }
    if !nodes.is_empty() || !structures.is_empty() {
        return Err(invalid("unreferenced node or structure relation"));
    }
    dependencies.sort_by_key(|edge| (edge.prerequisite, edge.dependent));
    let mut plan = Plan {
        id: snapshot.plan,
        session: payload.session,
        anchor: payload.anchor,
        goal: payload.goal,
        revision: payload.mutation_revision,
        status: payload.status,
        root_branch: payload.root_branch,
        nodes: plan_nodes,
        dependencies,
        coordinator: payload.coordinator,
        coordinator_cursor: payload.coordinator_cursor,
        coordinator_claim: payload.coordinator_claim,
        coordinator_generation: payload.coordinator_generation,
        root_nodes,
        history,
    };
    for record in &mut plan.history {
        if let Some(reason) = history_metadata.get(&record.revision) {
            record.reason = reason.clone();
        }
    }
    Ok(plan)
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
