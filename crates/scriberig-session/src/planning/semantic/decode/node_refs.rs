//! Queries plan membership and structure without decoding node payloads.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::{EntityId, RevisionId};

use super::super::super::{PlanNodeParent, PlanNodeStatus};
use super::super::encode::{
    MEMBERSHIP_TYPE, NODE_TYPE, PLAN_TYPE, RelationIdentity, STRUCTURE_TYPE, parent_identity,
    relation_id,
};
use super::super::{PlanSemanticError, PlanSemanticNodeRef, PlanSemanticSnapshot};
use super::{
    MembershipPayload, NodeStatusPayload, StructurePayload, decode_payload, invalid,
    validate_revision,
};

/// Lists `PlanNode` revisions using only identities and relation payloads.
///
/// - `snapshot`: Exact semantic plan object set.
///
/// Ordered node references with revision-bound structural parents.
///
/// Returns an error when membership or structure relations are invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn node_refs(
    snapshot: &PlanSemanticSnapshot,
) -> Result<Vec<PlanSemanticNodeRef>, PlanSemanticError> {
    let mut plan_objects = 0;
    let mut node_revisions = BTreeMap::new();
    let mut node_statuses = BTreeMap::new();
    let mut memberships = BTreeMap::new();
    let mut structures = BTreeMap::new();
    for object in &snapshot.objects {
        match object.object_type.as_str() {
            PLAN_TYPE => {
                validate_revision(object)?;
                if object.entity != snapshot.plan {
                    return Err(invalid("mismatched plan object"));
                }
                plan_objects += 1;
            }
            NODE_TYPE => {
                add_node(snapshot, object, &mut node_revisions, &mut node_statuses)?;
            }
            MEMBERSHIP_TYPE => {
                add_membership(snapshot, object, &mut memberships)?;
            }
            STRUCTURE_TYPE => {
                add_structure(snapshot, object, &mut structures)?;
            }
            _ => {}
        }
    }
    if plan_objects != 1 || node_revisions.len() != memberships.len() {
        return Err(invalid("plan, node, and membership sets differ"));
    }
    let mut result = Vec::with_capacity(node_revisions.len());
    let mut seen_ordinals = BTreeSet::new();
    for (entity, ordinal) in memberships {
        let Some(revision) = node_revisions.get(&entity) else {
            return Err(invalid("membership points to a missing PlanNode"));
        };
        let Some(status) = node_statuses.get(&entity) else {
            return Err(invalid("PlanNode status is missing"));
        };
        if !seen_ordinals.insert(ordinal) {
            return Err(invalid("duplicate plan membership ordinal"));
        }
        let Some((structure_ordinal, parent)) = structures.remove(&entity) else {
            return Err(invalid("PlanNode has no structural relation"));
        };
        if structure_ordinal != ordinal {
            return Err(invalid("membership and structure order disagree"));
        }
        result.push(PlanSemanticNodeRef {
            entity,
            revision: *revision,
            status: *status,
            parent,
            ordinal,
        });
    }
    if !structures.is_empty() {
        return Err(invalid("structure points to a missing membership"));
    }
    result.sort_by_key(|node| (node.ordinal, node.entity));
    Ok(result)
}

/// Adds a validated node revision and its direct status to the node maps.
///
/// # Parameters
/// - `snapshot`: Exact plan object set used for ownership validation.
/// - `object`: Candidate `PlanNode` revision.
/// - `revisions`: Node identity to exact revision map.
/// - `statuses`: Node identity to canonical lifecycle status map.
///
/// # Errors
/// Returns an error when identity, payload, or revision validation fails.
/// # Returns
/// Unit after the node has been added to both maps.
fn add_node(
    snapshot: &PlanSemanticSnapshot,
    object: &super::super::PlanSemanticRevision,
    revisions: &mut BTreeMap<EntityId, RevisionId>,
    statuses: &mut BTreeMap<EntityId, PlanNodeStatus>,
) -> Result<(), PlanSemanticError> {
    validate_revision(object)?;
    let payload: NodeStatusPayload = decode_payload(object)?;
    if payload.schema_version != 1 || payload.plan != snapshot.plan {
        return Err(invalid("mismatched plan node payload"));
    }
    if revisions.insert(object.entity, object.revision).is_some() {
        return Err(invalid("duplicate PlanNode identity"));
    }
    statuses.insert(object.entity, payload.status);
    Ok(())
}

/// Adds a validated plan membership relation.
///
/// # Parameters
/// - `snapshot`: Exact plan object set used for ownership validation.
/// - `object`: Candidate membership relation revision.
/// - `memberships`: Node identity to stable insertion order map.
///
/// # Errors
/// Returns an error when the relation payload or derived identity is invalid.
/// # Returns
/// Unit after the membership relation has been validated and added.
fn add_membership(
    snapshot: &PlanSemanticSnapshot,
    object: &super::super::PlanSemanticRevision,
    memberships: &mut BTreeMap<EntityId, u64>,
) -> Result<(), PlanSemanticError> {
    validate_revision(object)?;
    let payload: MembershipPayload = decode_payload(object)?;
    let expected = relation_id(
        MEMBERSHIP_TYPE,
        &RelationIdentity {
            owner: payload.plan,
            target: payload.node,
        },
    )?;
    if payload.schema_version != 1
        || payload.plan != snapshot.plan
        || expected != object.entity
        || memberships.insert(payload.node, payload.ordinal).is_some()
    {
        return Err(invalid("duplicate or mismatched plan membership"));
    }
    Ok(())
}

/// Adds a validated structural relation for one plan node.
///
/// # Parameters
/// - `snapshot`: Exact plan object set used for ownership validation.
/// - `object`: Candidate structural relation revision.
/// - `structures`: Node identity to order and parent map.
///
/// # Errors
/// Returns an error when the relation payload or derived identity is invalid.
/// # Returns
/// Unit after the structure relation has been validated and added.
fn add_structure(
    snapshot: &PlanSemanticSnapshot,
    object: &super::super::PlanSemanticRevision,
    structures: &mut BTreeMap<EntityId, (u64, Option<PlanNodeParent>)>,
) -> Result<(), PlanSemanticError> {
    validate_revision(object)?;
    let payload: StructurePayload = decode_payload(object)?;
    let expected = relation_id(
        STRUCTURE_TYPE,
        &RelationIdentity {
            owner: payload.child,
            target: payload.parent.map_or(payload.root_branch, parent_identity),
        },
    )?;
    if payload.schema_version != 1
        || payload.plan != snapshot.plan
        || expected != object.entity
        || structures
            .insert(payload.child, (payload.ordinal, payload.parent))
            .is_some()
    {
        return Err(invalid("duplicate or mismatched plan structure"));
    }
    Ok(())
}
