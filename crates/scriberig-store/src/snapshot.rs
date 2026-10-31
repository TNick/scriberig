//! Defines immutable persistent radix snapshots and deterministic diffs.

use std::collections::{BTreeMap, BTreeSet};

#[path = "snapshot_build.rs"]
mod snapshot_build;
#[path = "snapshot_diff.rs"]
mod snapshot_diff;
#[path = "snapshot_node.rs"]
mod snapshot_node;
#[path = "snapshot_relation_state.rs"]
mod snapshot_relation_state;
#[path = "snapshot_types.rs"]
mod snapshot_types;
use self::snapshot_node::Node;
pub use self::snapshot_types::{
    SnapshotChange, SnapshotChangeProvenance, SnapshotDiff, SnapshotError,
};
use crate::snapshot_relation::{SnapshotRelation, SnapshotRelationChange};
use scriberig_core::{EncodingVersion, EntityId, RevisionId, SnapshotId};
use std::sync::Arc;

/// Internal snapshot operation.
/// - `entity`: Operation input.
/// - `depth`: Operation input.
///
/// The requested snapshot result.
/// # Returns
/// The operation result.
fn nibble(entity: EntityId, depth: usize) -> usize {
    let byte = entity.as_bytes()[depth / 2];
    usize::from(if depth % 2 == 0 {
        byte >> 4
    } else {
        byte & 0x0f
    })
}

/// Internal snapshot operation.
/// - `node`: Operation input.
/// - `entity`: Operation input.
/// - `revision`: Operation input.
/// - `depth`: Operation input.
///
/// The requested snapshot result.
/// # Returns
/// The operation result.
fn insert(node: &Arc<Node>, entity: EntityId, revision: RevisionId, depth: usize) -> Arc<Node> {
    if depth == 32 {
        return Arc::new(Node::Leaf(entity, revision));
    }
    let mut children = match node.as_ref() {
        Node::Branch(children) => children.clone(),
        _ => Node::empty_branch(),
    };
    let index = nibble(entity, depth);
    let child = children[index]
        .clone()
        .unwrap_or_else(|| Arc::new(Node::Empty));
    children[index] = Some(insert(&child, entity, revision, depth + 1));
    Arc::new(Node::Branch(children))
}

/// Internal snapshot operation.
/// - `node`: Operation input.
/// - `entity`: Operation input.
/// - `depth`: Operation input.
///
/// The requested snapshot result.
/// # Returns
/// The operation result.
fn remove(node: &Arc<Node>, entity: EntityId, depth: usize) -> Arc<Node> {
    if depth == 32 {
        return Arc::new(Node::Empty);
    }
    let Node::Branch(existing) = node.as_ref() else {
        return node.clone();
    };
    let mut children = existing.clone();
    let index = nibble(entity, depth);
    if let Some(child) = &children[index] {
        let replacement = remove(child, entity, depth + 1);
        children[index] = match replacement.as_ref() {
            Node::Empty => None,
            Node::Branch(_) | Node::Leaf(_, _) => Some(replacement),
        };
    }
    if children.iter().all(Option::is_none) {
        Arc::new(Node::Empty)
    } else {
        Arc::new(Node::Branch(children))
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;

/// Internal snapshot operation.
/// - `node`: Operation input.
/// - `entity`: Operation input.
/// - `depth`: Operation input.
///
/// The requested snapshot result.
/// # Returns
/// The operation result.
fn resolve(node: &Arc<Node>, entity: EntityId, depth: usize) -> Option<RevisionId> {
    match node.as_ref() {
        Node::Leaf(found, revision) if *found == entity => Some(*revision),
        Node::Branch(children) if depth < 32 => children[nibble(entity, depth)]
            .as_ref()
            .and_then(|child| resolve(child, entity, depth + 1)),
        _ => None,
    }
}

/// Internal snapshot operation.
/// - `node`: Operation input.
/// - `values`: Operation input.
/// - `RevisionId`: Revision component of each collected pair.
fn collect(node: &Arc<Node>, values: &mut Vec<(EntityId, RevisionId)>) {
    match node.as_ref() {
        Node::Leaf(entity, revision) => values.push((*entity, *revision)),
        Node::Branch(children) => {
            for child in children.iter().flatten() {
                collect(child, values);
            }
        }
        Node::Empty => {}
    }
}

/// An immutable entity-to-revision mapping backed by a persistent radix tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    version: EncodingVersion,
    root: Arc<Node>,
    id: SnapshotId,
    len: usize,
    relations: BTreeSet<SnapshotRelation>,
}

impl Snapshot {
    ///
    /// The requested snapshot result.
    /// Creates an empty version-one snapshot.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn empty() -> Self {
        Self::from_root(
            EncodingVersion::V1,
            Arc::new(Node::Empty),
            0,
            BTreeSet::new(),
        )
    }
    /// - `version`: Operation input.
    ///
    /// The requested snapshot result.
    ///
    /// Returns `InvalidEncoding` when the requested version is unsupported.
    /// Creates an empty snapshot at an encoding version.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn empty_with_version(version: EncodingVersion) -> Result<Self, SnapshotError> {
        EncodingVersion::try_new(version.value()).map_err(SnapshotError::InvalidEncoding)?;
        Ok(Self::from_root(
            version,
            Arc::new(Node::Empty),
            0,
            BTreeSet::new(),
        ))
    }
    /// - `entries`: Operation input.
    ///
    /// The requested snapshot result.
    /// Creates a canonical snapshot from entries.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_entries(entries: BTreeMap<EntityId, RevisionId>) -> Self {
        let entries = entries.into_iter().collect::<Vec<_>>();
        Self::from_sorted_entries(EncodingVersion::V1, &entries)
    }

    /// Creates a snapshot from sorted entries at an explicit encoding version.
    ///
    /// - `version`: Snapshot encoding version.
    /// - `entries`: Entity/revision pairs in ascending entity order.
    /// - `RevisionId`: Revision component of each entry.
    ///
    /// A canonical immutable snapshot.
    /// # Returns
    /// The operation result.
    fn from_sorted_entries(version: EncodingVersion, entries: &[(EntityId, RevisionId)]) -> Self {
        let len = entries.len();
        Self::from_root(
            version,
            snapshot_build::build(entries, 0),
            len,
            BTreeSet::new(),
        )
    }
    /// Internal snapshot operation.
    /// - `version`: Operation input.
    /// - `root`: Operation input.
    /// - `len`: Operation input.
    /// - `relations`: Canonical typed relations carried by the snapshot.
    ///
    /// The requested snapshot result.
    /// # Returns
    /// The operation result.
    fn from_root(
        version: EncodingVersion,
        root: Arc<Node>,
        len: usize,
        relations: BTreeSet<SnapshotRelation>,
    ) -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"scriberig.snapshot.root.v1");
        bytes.extend_from_slice(&version.value().to_be_bytes());
        bytes.extend_from_slice(&root.hash(version));
        if !relations.is_empty() {
            bytes.extend_from_slice(b"scriberig.snapshot.relations.v1");
            for relation in &relations {
                bytes.extend_from_slice(relation.from.as_bytes());
                let relation_type = relation.relation_type.as_str().as_bytes();
                bytes.extend_from_slice(&(relation_type.len() as u64).to_be_bytes());
                bytes.extend_from_slice(relation_type);
                bytes.extend_from_slice(relation.to.as_bytes());
            }
        }
        let id = SnapshotId::from_bytes(*blake3::hash(&bytes).as_bytes());
        Self {
            version,
            root,
            id,
            len,
            relations,
        }
    }
    ///
    /// The requested snapshot result.
    /// Returns the content-derived snapshot identity.
    #[must_use]
    pub const fn id(&self) -> SnapshotId {
        self.id
    }
    ///
    /// The requested snapshot result.
    /// Returns the number of mapped entities.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Reports whether this snapshot contains no entities.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// - `entity`: Operation input.
    ///
    /// The requested snapshot result.
    /// Resolves an entity only within this snapshot.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn resolve(&self, entity: EntityId) -> Option<RevisionId> {
        resolve(&self.root, entity, 0)
    }
    /// - `entity`: Operation input.
    /// - `revision`: Operation input.
    ///
    /// The requested snapshot result.
    /// Returns a successor sharing unaffected radix branches.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_revision(&self, entity: EntityId, revision: RevisionId) -> Self {
        if self.resolve(entity) == Some(revision) {
            return self.clone();
        }
        let len = self.len + usize::from(self.resolve(entity).is_none());
        Self::from_root(
            self.version,
            insert(&self.root, entity, revision, 0),
            len,
            self.relations.clone(),
        )
    }
    /// - `entity`: Operation input.
    ///
    /// The requested snapshot result.
    /// Returns a successor without one entity.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn without_entity(&self, entity: EntityId) -> Self {
        if self.resolve(entity).is_none() {
            return self.clone();
        }
        let relations = self
            .relations
            .iter()
            .filter(|r| r.from != entity && r.to != entity)
            .cloned()
            .collect();
        Self::from_root(
            self.version,
            remove(&self.root, entity, 0),
            self.len - 1,
            relations,
        )
    }
    ///
    /// The requested snapshot result.
    /// Iterates mappings in ascending entity order.
    /// # Returns
    /// The operation result.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, RevisionId)> {
        let mut values = Vec::new();
        collect(&self.root, &mut values);
        values.into_iter()
    }

    /// - `changes`: Operation input.
    ///
    /// The requested snapshot result.
    /// Applies storage changes and returns a persistent successor.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn apply(&self, changes: impl IntoIterator<Item = SnapshotChange>) -> Self {
        self.apply_changes_with_relations(changes, std::iter::empty())
    }

    /// Applies entity and relation changes to one immutable snapshot atomically.
    ///
    /// - `entity_changes`: Entity-to-revision additions and removals.
    /// - `relation_changes`: Typed relation additions and removals.
    ///
    /// # Returns
    /// A persistent successor containing both change sets.
    #[must_use]
    pub fn apply_changes_with_relations(
        &self,
        entity_changes: impl IntoIterator<Item = SnapshotChange>,
        relation_changes: impl IntoIterator<Item = SnapshotRelationChange>,
    ) -> Self {
        let mut entries = self.iter().collect::<BTreeMap<_, _>>();
        let mut relations = self.relations.clone();
        let mut removed_entities = BTreeSet::new();
        for change in entity_changes {
            match change {
                SnapshotChange::Set(entity, revision) => {
                    entries.insert(entity, revision);
                }
                SnapshotChange::Remove(entity) => {
                    entries.remove(&entity);
                    removed_entities.insert(entity);
                }
            }
        }
        for change in relation_changes {
            match change {
                SnapshotRelationChange::Add(relation) => {
                    relations.insert(relation);
                }
                SnapshotRelationChange::Remove(relation) => {
                    relations.remove(&relation);
                }
            }
        }
        removed_entities.retain(|entity| !entries.contains_key(entity));
        relations.retain(|relation| {
            !removed_entities.contains(&relation.from) && !removed_entities.contains(&relation.to)
        });
        let entries = entries.into_iter().collect::<Vec<_>>();
        Self::from_root(
            self.version,
            snapshot_build::build(&entries, 0),
            entries.len(),
            relations,
        )
    }
}
