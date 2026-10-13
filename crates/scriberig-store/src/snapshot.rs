//! Defines immutable persistent radix snapshots and deterministic diffs.

use std::collections::BTreeMap;

#[path = "snapshot_build.rs"]
mod snapshot_build;
#[path = "snapshot_diff.rs"]
mod snapshot_diff;
use std::sync::Arc;

use scriberig_core::{EncodingError, EncodingVersion, EntityId, RevisionId, SnapshotId};

/// Describes one storage-level snapshot update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotChange {
    /// Associates an entity with a revision.
    Set(EntityId, RevisionId),
    /// Removes an entity association.
    Remove(EntityId),
}

/// Identifies the source change that produced one immutable snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotChangeProvenance {
    /// Stable identity of the accepted source change.
    pub change_id: String,
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
    /// The requested canonical encoding version is not supported.
    InvalidEncoding(EncodingError),
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
        Self::from_root(EncodingVersion::V1, Arc::new(Node::Empty), 0)
    }
    /// # Parameters
    /// - `version`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    ///
    /// # Errors
    /// Returns `InvalidEncoding` when the requested version is unsupported.
    /// Creates an empty snapshot at an encoding version.
    pub fn empty_with_version(version: EncodingVersion) -> Result<Self, SnapshotError> {
        EncodingVersion::try_new(version.value()).map_err(SnapshotError::InvalidEncoding)?;
        Ok(Self::from_root(version, Arc::new(Node::Empty), 0))
    }
    /// # Parameters
    /// - `entries`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    /// Creates a canonical snapshot from entries.
    #[must_use]
    pub fn from_entries(entries: BTreeMap<EntityId, RevisionId>) -> Self {
        let entries = entries.into_iter().collect::<Vec<_>>();
        Self::from_sorted_entries(EncodingVersion::V1, &entries)
    }

    /// Creates a snapshot from sorted entries at an explicit encoding version.
    ///
    /// # Parameters
    /// - `version`: Snapshot encoding version.
    /// - `entries`: Entity/revision pairs in ascending entity order.
    /// - `RevisionId`: Revision component of each entry.
    ///
    /// # Returns
    /// A canonical immutable snapshot.
    fn from_sorted_entries(version: EncodingVersion, entries: &[(EntityId, RevisionId)]) -> Self {
        let len = entries.len();
        Self::from_root(version, snapshot_build::build(entries, 0), len)
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
        let mut entries = self.iter().collect::<BTreeMap<_, _>>();
        for change in changes {
            match change {
                SnapshotChange::Set(entity, revision) => {
                    entries.insert(entity, revision);
                }
                SnapshotChange::Remove(entity) => {
                    entries.remove(&entity);
                }
            }
        }
        let entries = entries.into_iter().collect::<Vec<_>>();
        Self::from_sorted_entries(self.version, &entries)
    }
}
