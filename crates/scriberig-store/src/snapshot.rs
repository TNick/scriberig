//! Defines immutable persistent radix snapshots and deterministic diffs.

use std::collections::BTreeMap;
use std::sync::Arc;

use scriberig_core::{EncodingVersion, EntityId, RevisionId, SnapshotId};

/// Describes one storage-level snapshot update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotChange {
    /// Associates an entity with a revision.
    Set(EntityId, RevisionId),
    /// Removes an entity association.
    Remove(EntityId),
}

/// Describes one semantic difference between two immutable snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotDiff {
    /// An entity is present only in the target snapshot.
    Added {
        /// Entity introduced by the target snapshot.
        entity: EntityId,
        /// Revision assigned by the target snapshot.
        revision: RevisionId,
    },
    /// An entity is present only in the source snapshot.
    Removed {
        /// Entity removed by the target snapshot.
        entity: EntityId,
        /// Revision assigned by the source snapshot.
        revision: RevisionId,
    },
    /// An entity resolves to a different revision in the target snapshot.
    Changed {
        /// Entity whose resolution changed.
        entity: EntityId,
        /// Revision assigned by the source snapshot.
        from: RevisionId,
        /// Revision assigned by the target snapshot.
        to: RevisionId,
    },
}

/// Reports an invalid snapshot operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    /// The requested snapshot is not known by the backend.
    UnknownSnapshot(SnapshotId),
    /// A backend-specific durable storage failure.
    Backend(String),
}

/// Private persistent radix node shared by related snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Node {
    Empty,
    Leaf(EntityId, RevisionId),
    Branch(Vec<Option<Arc<Node>>>),
}

impl Node {
    /// Internal snapshot operation.
    ///
    /// # Returns
    /// The requested snapshot result.
    fn empty_branch() -> Vec<Option<Arc<Node>>> {
        vec![None; 16]
    }

    /// Internal snapshot operation.
    /// # Parameters
    /// - `version`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    fn hash(&self, version: EncodingVersion) -> [u8; 32] {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&version.value().to_be_bytes());
        match self {
            Self::Empty => bytes.extend_from_slice(b"scriberig.trie.empty.v1"),
            Self::Leaf(entity, revision) => {
                bytes.extend_from_slice(b"scriberig.trie.leaf.v1");
                bytes.extend_from_slice(entity.as_bytes());
                bytes.extend_from_slice(revision.as_bytes());
            }
            Self::Branch(children) => {
                bytes.extend_from_slice(b"scriberig.trie.branch.v1");
                for child in children {
                    let hash = child
                        .as_ref()
                        .map_or_else(|| Self::Empty.hash(version), |node| node.hash(version));
                    bytes.extend_from_slice(&hash);
                }
            }
        }
        *blake3::hash(&bytes).as_bytes()
    }
}

/// Internal snapshot operation.
/// # Parameters
/// - `entity`: Operation input.
/// - `depth`: Operation input.
///
/// # Returns
/// The requested snapshot result.
fn nibble(entity: EntityId, depth: usize) -> usize {
    let byte = entity.as_bytes()[depth / 2];
    usize::from(if depth % 2 == 0 {
        byte >> 4
    } else {
        byte & 0x0f
    })
}

/// Internal snapshot operation.
/// # Parameters
/// - `node`: Operation input.
/// - `entity`: Operation input.
/// - `revision`: Operation input.
/// - `depth`: Operation input.
///
/// # Returns
/// The requested snapshot result.
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
/// # Parameters
/// - `node`: Operation input.
/// - `entity`: Operation input.
/// - `depth`: Operation input.
///
/// # Returns
/// The requested snapshot result.
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
/// # Parameters
/// - `node`: Operation input.
/// - `entity`: Operation input.
/// - `depth`: Operation input.
///
/// # Returns
/// The requested snapshot result.
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
/// # Parameters
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
}

impl Snapshot {
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Creates an empty version-one snapshot.
    #[must_use]
    pub fn empty() -> Self {
        Self::empty_with_version(EncodingVersion::new(1))
    }
    /// # Parameters
    /// - `version`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Creates an empty snapshot at an encoding version.
    #[must_use]
    pub fn empty_with_version(version: EncodingVersion) -> Self {
        Self::from_root(version, Arc::new(Node::Empty), 0)
    }
    /// # Parameters
    /// - `entries`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Creates a canonical snapshot from entries.
    #[must_use]
    pub fn from_entries(entries: BTreeMap<EntityId, RevisionId>) -> Self {
        entries
            .into_iter()
            .fold(Self::empty(), |snapshot, (entity, revision)| {
                snapshot.with_revision(entity, revision)
            })
    }
    /// Internal snapshot operation.
    /// # Parameters
    /// - `version`: Operation input.
    /// - `root`: Operation input.
    /// - `len`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    fn from_root(version: EncodingVersion, root: Arc<Node>, len: usize) -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"scriberig.snapshot.root.v1");
        bytes.extend_from_slice(&version.value().to_be_bytes());
        bytes.extend_from_slice(&root.hash(version));
        let id = SnapshotId::from_bytes(*blake3::hash(&bytes).as_bytes());
        Self {
            version,
            root,
            id,
            len,
        }
    }
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Returns the content-derived snapshot identity.
    #[must_use]
    pub const fn id(&self) -> SnapshotId {
        self.id
    }
    ///
    /// # Returns
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
    /// # Parameters
    /// - `entity`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Resolves an entity only within this snapshot.
    #[must_use]
    pub fn resolve(&self, entity: EntityId) -> Option<RevisionId> {
        resolve(&self.root, entity, 0)
    }
    /// # Parameters
    /// - `entity`: Operation input.
    /// - `revision`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Returns a successor sharing unaffected radix branches.
    #[must_use]
    pub fn with_revision(&self, entity: EntityId, revision: RevisionId) -> Self {
        if self.resolve(entity) == Some(revision) {
            return self.clone();
        }
        let len = self.len + usize::from(self.resolve(entity).is_none());
        Self::from_root(self.version, insert(&self.root, entity, revision, 0), len)
    }
    /// # Parameters
    /// - `entity`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Returns a successor without one entity.
    #[must_use]
    pub fn without_entity(&self, entity: EntityId) -> Self {
        if self.resolve(entity).is_none() {
            return self.clone();
        }
        Self::from_root(self.version, remove(&self.root, entity, 0), self.len - 1)
    }
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Iterates mappings in ascending entity order.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, RevisionId)> {
        let mut values = Vec::new();
        collect(&self.root, &mut values);
        values.into_iter()
    }
    /// # Parameters
    /// - `changes`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Applies storage changes and returns a persistent successor.
    #[must_use]
    pub fn apply(&self, changes: impl IntoIterator<Item = SnapshotChange>) -> Self {
        changes
            .into_iter()
            .fold(self.clone(), |snapshot, change| match change {
                SnapshotChange::Set(entity, revision) => snapshot.with_revision(entity, revision),
                SnapshotChange::Remove(entity) => snapshot.without_entity(entity),
            })
    }
    /// # Parameters
    /// - `other`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Computes explicit semantic differences in ascending entity order.
    #[must_use]
    pub fn diff(&self, other: &Self) -> Vec<SnapshotDiff> {
        self.diff_semantic(other)
    }

    /// Computes storage-level changes suitable for applying to another snapshot.
    #[must_use]
    pub fn diff_changes(&self, other: &Self) -> Vec<SnapshotChange> {
        self.diff_semantic(other)
            .into_iter()
            .map(|change| match change {
                SnapshotDiff::Added { entity, revision }
                | SnapshotDiff::Changed {
                    entity,
                    to: revision,
                    ..
                } => SnapshotChange::Set(entity, revision),
                SnapshotDiff::Removed { entity, .. } => SnapshotChange::Remove(entity),
            })
            .collect()
    }

    /// Computes explicit semantic differences in ascending entity order.
    #[must_use]
    pub fn diff_semantic(&self, other: &Self) -> Vec<SnapshotDiff> {
        let left: BTreeMap<_, _> = self.iter().collect();
        let right: BTreeMap<_, _> = other.iter().collect();
        left.keys()
            .chain(right.keys())
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter_map(|entity| match (left.get(&entity), right.get(&entity)) {
                (None, Some(revision)) => Some(SnapshotDiff::Added {
                    entity,
                    revision: *revision,
                }),
                (Some(revision), None) => Some(SnapshotDiff::Removed {
                    entity,
                    revision: *revision,
                }),
                (Some(from), Some(to)) if from != to => Some(SnapshotDiff::Changed {
                    entity,
                    from: *from,
                    to: *to,
                }),
                _ => None,
            })
            .collect()
    }
}
