//! Builds typed immutable object payloads from the current plan projection.

use scriberig_core::{EncodingVersion, EntityId, TypeId, revision_id};
use serde::Serialize;

use super::super::Plan;
use super::{PlanSemanticError, PlanSemanticRevision, PlanSemanticSnapshot};

pub(super) const PLAN_TYPE: &str = "scriberig.session.plan";
pub(super) const NODE_TYPE: &str = "scriberig.session.plan-node";
pub(super) const MEMBERSHIP_TYPE: &str = "scriberig.session.plan-membership";
pub(super) const STRUCTURE_TYPE: &str = "scriberig.session.plan-structure";
pub(super) const DEPENDENCY_TYPE: &str = "scriberig.session.plan-dependency";

#[derive(Serialize)]
struct PlanPayload<'a> {
    schema_version: u16,
    session: Option<EntityId>,
    anchor: Option<&'a crate::ContinuationAnchor>,
    goal: &'a str,
    mutation_revision: u64,
    status: super::super::PlanStatus,
    root_branch: EntityId,
    coordinator: Option<EntityId>,
    coordinator_cursor: Option<EntityId>,
    coordinator_claim: Option<&'a super::super::CoordinatorClaim>,
    coordinator_generation: u64,
}

#[derive(Serialize)]
struct NodePayload<'a> {
    schema_version: u16,
    plan: EntityId,
    title: &'a str,
    purpose: &'a str,
    expected_result: &'a Option<super::super::ExpectedResult>,
    semantic_scope: &'a [String],
    status: super::super::PlanNodeStatus,
    branch: Option<EntityId>,
    result: Option<EntityId>,
    superseded_by: Option<EntityId>,
    assignment: &'a Option<super::super::ModelAssignment>,
    claim: &'a Option<super::super::WorkerClaim>,
    claim_generation: u64,
}

#[derive(Serialize)]
struct MembershipPayload {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    ordinal: u64,
}

#[derive(Serialize)]
struct StructurePayload {
    schema_version: u16,
    plan: EntityId,
    root_branch: EntityId,
    child: EntityId,
    parent: Option<super::super::PlanNodeParent>,
    ordinal: u64,
}

#[derive(Serialize)]
struct DependencyPayload {
    schema_version: u16,
    plan: EntityId,
    prerequisite: EntityId,
    dependent: EntityId,
    fulfilled_by: Option<EntityId>,
}

#[derive(Serialize)]
pub(super) struct RelationIdentity {
    pub(super) owner: EntityId,
    pub(super) target: EntityId,
}

#[derive(Serialize)]
pub(super) struct DependencyIdentity {
    pub(super) plan: EntityId,
    pub(super) prerequisite: EntityId,
    pub(super) dependent: EntityId,
}

/// Encodes a plan view into current independently revisioned objects.
///
/// - `plan`: Aggregate read model to split into semantic records.
///
/// A stable object set sorted by entity and type identity.
///
/// Returns a serialization or canonical identity error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn from_plan(plan: &Plan) -> Result<PlanSemanticSnapshot, PlanSemanticError> {
    let mut objects = Vec::new();
    push_object(
        &mut objects,
        plan.id,
        PLAN_TYPE,
        &PlanPayload {
            schema_version: 1,
            session: plan.session,
            anchor: plan.anchor.as_ref(),
            goal: &plan.goal,
            mutation_revision: plan.revision,
            status: plan.status,
            root_branch: plan.root_branch,
            coordinator: plan.coordinator,
            coordinator_cursor: plan.coordinator_cursor,
            coordinator_claim: plan.coordinator_claim.as_ref(),
            coordinator_generation: plan.coordinator_generation,
        },
    )?;

    for (ordinal, node) in plan.nodes.iter().enumerate() {
        push_object(
            &mut objects,
            node.id,
            NODE_TYPE,
            &NodePayload {
                schema_version: 1,
                plan: plan.id,
                title: &node.title,
                purpose: &node.purpose,
                expected_result: &node.expected_result,
                semantic_scope: &node.semantic_scope,
                status: node.status,
                branch: node.branch,
                result: node.result,
                superseded_by: node.superseded_by,
                assignment: &node.assignment,
                claim: &node.claim,
                claim_generation: node.claim_generation,
            },
        )?;
        let ordinal = u64::try_from(ordinal).map_err(|error| {
            PlanSemanticError::Serialization(format!("invalid node ordinal: {error}"))
        })?;
        let membership = MembershipPayload {
            schema_version: 1,
            plan: plan.id,
            node: node.id,
            ordinal,
        };
        push_relation(
            &mut objects,
            MEMBERSHIP_TYPE,
            &RelationIdentity {
                owner: plan.id,
                target: node.id,
            },
            &membership,
        )?;
        let structure = StructurePayload {
            schema_version: 1,
            plan: plan.id,
            root_branch: plan.root_branch,
            child: node.id,
            parent: node.parent,
            ordinal,
        };
        push_relation(
            &mut objects,
            STRUCTURE_TYPE,
            &RelationIdentity {
                owner: node.id,
                target: node.parent.map_or(plan.root_branch, parent_identity),
            },
            &structure,
        )?;
    }

    for dependency in &plan.dependencies {
        let payload = DependencyPayload {
            schema_version: 1,
            plan: plan.id,
            prerequisite: dependency.prerequisite,
            dependent: dependency.dependent,
            fulfilled_by: dependency.fulfilled_by,
        };
        push_relation(
            &mut objects,
            DEPENDENCY_TYPE,
            &DependencyIdentity {
                plan: plan.id,
                prerequisite: dependency.prerequisite,
                dependent: dependency.dependent,
            },
            &payload,
        )?;
    }

    objects.sort_by(|left, right| {
        (&left.entity, left.object_type.as_str()).cmp(&(&right.entity, right.object_type.as_str()))
    });
    Ok(PlanSemanticSnapshot {
        plan: plan.id,
        objects,
    })
}

/// Encodes one typed semantic object and computes its immutable revision ID.
///
/// - `objects`: Destination object set.
/// - `entity`: Stable entity identity.
/// - `type_name`: Stable semantic object type.
/// - `value`: Versioned typed payload.
///
/// Unit after the record has been appended.
///
/// Returns an encoding or identity error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn push_object<T: Serialize>(
    objects: &mut Vec<PlanSemanticRevision>,
    entity: EntityId,
    type_name: &str,
    value: &T,
) -> Result<(), PlanSemanticError> {
    let object_type = TypeId::new(type_name)
        .map_err(|error| PlanSemanticError::InvalidObject(error.to_string()))?;
    let payload = serde_json::to_vec(value)
        .map_err(|error| PlanSemanticError::Serialization(format!("{error:?}")))?;
    let revision = revision_id(EncodingVersion::V1, &object_type, &payload)
        .map_err(|error| PlanSemanticError::Serialization(format!("{error:?}")))?;
    objects.push(PlanSemanticRevision {
        entity,
        object_type,
        revision,
        payload,
    });
    Ok(())
}

/// Creates a stable relation identity from its immutable key.
///
/// - `type_name`: Relation object type.
/// - `key`: Stable source/target identity tuple.
///
/// The relation's stable entity identity.
///
/// Returns an encoding error if the key cannot be represented.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn relation_id<T: Serialize>(
    type_name: &str,
    key: &T,
) -> Result<EntityId, PlanSemanticError> {
    let object_type = TypeId::new(type_name)
        .map_err(|error| PlanSemanticError::InvalidObject(error.to_string()))?;
    let payload = serde_json::to_vec(key)
        .map_err(|error| PlanSemanticError::Serialization(error.to_string()))?;
    let revision = revision_id(EncodingVersion::V1, &object_type, &payload)
        .map_err(|error| PlanSemanticError::Serialization(format!("{error:?}")))?;
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&revision.as_bytes()[..16]);
    Ok(EntityId::from_bytes(bytes))
}

/// Appends one relation represented as a stable semantic entity.
///
/// - `objects`: Destination object set.
/// - `type_name`: Stable relation entity type.
/// - `key`: Relation identity that remains stable across payload changes.
/// - `payload`: Current relation fields.
///
/// Unit after the relation revision is appended.
///
/// Returns an encoding or identity error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn push_relation<K: Serialize, T: Serialize>(
    objects: &mut Vec<PlanSemanticRevision>,
    type_name: &str,
    key: &K,
    payload: &T,
) -> Result<(), PlanSemanticError> {
    let entity = relation_id(type_name, key)?;
    push_object(objects, entity, type_name, payload)
}

/// Converts a structural parent reference to the relation-key target ID.
///
/// - `parent`: Structural plan parent.
///
/// The stable parent entity identity.
/// # Returns
/// The operation result.
pub(super) fn parent_identity(parent: super::super::PlanNodeParent) -> EntityId {
    match parent {
        super::super::PlanNodeParent::Branch(branch) => branch,
        super::super::PlanNodeParent::Node(node) => node,
    }
}
