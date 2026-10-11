//! Defines generic and domain-extensible semantic operations.

use scriberig_core::{EntityId, KnowledgeValue, ObjectType, RevisionId};

/// Identifies a semantic collection position without file offsets.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CollectionPosition {
    /// Ordered collection index.
    pub index: u64,
}

/// Universal semantic change operations.
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    /// Creates an entity with an initial revision.
    CreateEntity {
        /// Created entity identity.
        entity: EntityId,
        /// Initial entity revision.
        revision: RevisionId,
    },
    /// Replaces an entity revision.
    ReplaceRevision {
        /// Changed entity identity.
        entity: EntityId,
        /// Replacement revision.
        revision: RevisionId,
    },
    /// Removes an entity from canonical state.
    RemoveEntity {
        /// Removed entity identity.
        entity: EntityId,
    },
    /// Adds a semantic relation.
    AddRelation {
        /// Source entity.
        from: EntityId,
        /// Stable relation name.
        relation: String,
        /// Target entity.
        to: EntityId,
    },
    /// Removes a semantic relation.
    RemoveRelation {
        /// Source entity.
        from: EntityId,
        /// Stable relation name.
        relation: String,
        /// Target entity.
        to: EntityId,
    },
    /// Inserts an ordered collection member.
    InsertCollectionMember {
        /// Collection entity.
        collection: EntityId,
        /// Inserted member.
        member: EntityId,
        /// Semantic insertion position.
        position: CollectionPosition,
    },
    /// Removes an ordered collection member.
    RemoveCollectionMember {
        /// Collection entity.
        collection: EntityId,
        /// Removed member.
        member: EntityId,
    },
    /// Moves an ordered collection member.
    MoveCollectionMember {
        /// Collection entity.
        collection: EntityId,
        /// Moved member.
        member: EntityId,
        /// Semantic destination position.
        position: CollectionPosition,
    },
    /// Replaces collection structure while preserving semantic references.
    ReplaceCollectionStructure {
        /// Collection entity.
        collection: EntityId,
        /// New ordered members.
        members: Vec<EntityId>,
    },
    /// Carries domain-specific semantic intent opaquely.
    Domain {
        /// Domain type namespace.
        domain: ObjectType,
        /// Domain operation identifier.
        operation: String,
        /// Opaque semantic payload.
        payload: KnowledgeValue,
    },
}
