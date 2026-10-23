//! Queries plan membership and structure without decoding node payloads.

use std::collections::{BTreeMap, BTreeSet};

use super::super::encode::{
    MEMBERSHIP_TYPE, NODE_TYPE, PLAN_TYPE, RelationIdentity, STRUCTURE_TYPE, parent_identity,
    relation_id,
};
use super::super::{PlanSemanticError, PlanSemanticNodeRef, PlanSemanticSnapshot};
use super::{MembershipPayload, StructurePayload, decode_payload, invalid, validate_revision};

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
pub(in crate::planning::semantic) fn node_refs(
    snapshot: &PlanSemanticSnapshot,
) -> Result<Vec<PlanSemanticNodeRef>, PlanSemanticError> {
    let mut plan_objects = 0;
    let mut node_revisions = BTreeMap::new();
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
                validate_revision(object)?;
                if node_revisions
                    .insert(object.entity, object.revision)
                    .is_some()
                {
                    return Err(invalid("duplicate PlanNode identity"));
                }
            }
            MEMBERSHIP_TYPE => {
                validate_revision(object)?;
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
                    return Err(invalid("duplicate or mismatched plan membership"));
                }
            }
            STRUCTURE_TYPE => {
                validate_revision(object)?;
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
                        .insert(
                            payload.child,
                            (payload.ordinal, payload.parent, payload.root_ordinal),
                        )
                        .is_some()
                {
                    return Err(invalid("duplicate or mismatched plan structure"));
                }
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
        if !seen_ordinals.insert(ordinal) {
            return Err(invalid("duplicate plan membership ordinal"));
        }
        let Some((structure_ordinal, parent, _root_ordinal)) = structures.remove(&entity) else {
            return Err(invalid("PlanNode has no structural relation"));
        };
        if structure_ordinal != ordinal {
            return Err(invalid("membership and structure order disagree"));
        }
        result.push(PlanSemanticNodeRef {
            entity,
            revision: *revision,
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
