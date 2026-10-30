//! Encodes immutable revisions and stable identities for semantic relations.

use scriberig_core::{EncodingVersion, EntityId, TypeId, revision_id};
use serde::Serialize;

use super::super::{PlanSemanticError, PlanSemanticRevision};

/// Encodes one typed semantic object and computes its immutable revision ID.
///
/// - `objects`: Destination object set.
/// - `entity`: Stable entity identity.
/// - `type_name`: Stable semantic object type.
/// - `value`: Versioned typed payload.
///
/// Unit after the record has been appended.
/// # Returns
/// Success after the immutable record is appended.
/// # Errors
/// Returns an encoding or identity error.
pub(in crate::planning::semantic) fn push_object<T: Serialize>(
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
/// # Returns
/// The derived relation entity identity.
/// # Errors
/// Returns an encoding error if the key cannot be represented.
pub(in crate::planning::semantic) fn relation_id<T: Serialize>(
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
/// # Returns
/// Success after the relation revision is appended.
/// # Errors
/// Returns an encoding or identity error.
pub(in crate::planning::semantic) fn push_relation<K: Serialize, T: Serialize>(
    objects: &mut Vec<PlanSemanticRevision>,
    type_name: &str,
    key: &K,
    payload: &T,
) -> Result<(), PlanSemanticError> {
    let entity = relation_id(type_name, key)?;
    push_object(objects, entity, type_name, payload)
}

/// Converts a structural parent reference to its relation-key target identity.
///
/// - `parent`: Structural plan parent.
///
/// The stable parent entity identity.
/// # Returns
/// The parent entity encoded in the structural relation.
pub(in crate::planning::semantic) fn parent_identity(
    parent: super::super::super::PlanNodeParent,
) -> EntityId {
    match parent {
        super::super::super::PlanNodeParent::Branch(branch) => branch,
        super::super::super::PlanNodeParent::Node(node) => node,
    }
}
