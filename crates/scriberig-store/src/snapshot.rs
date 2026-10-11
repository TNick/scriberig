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
                    bytes.extend_from_slice(
                        &child.as_ref().map_or([0; 32], |node| node.hash(version)),
                    );
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
        children[index] = Some(remove(child, entity, depth + 1));
    }
    if children.iter().all(Option::is_none) {
        Arc::new(Node::Empty)
    } else {
        Arc::new(Node::Branch(children))
    }
}

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
    /// Computes deterministic mapping changes from this snapshot to another.
    #[must_use]
    pub fn diff(&self, other: &Self) -> Vec<SnapshotChange> {
        let left: BTreeMap<_, _> = self.iter().collect();
        let right: BTreeMap<_, _> = other.iter().collect();
        let mut changes = Vec::new();
        for (entity, revision) in &right {
            if left.get(entity) != Some(revision) {
                changes.push(SnapshotChange::Set(*entity, *revision));
            }
        }
        for entity in left.keys() {
            if !right.contains_key(entity) {
                changes.push(SnapshotChange::Remove(*entity));
            }
        }
        changes
    }
}

/// Defines backend-neutral snapshot operations.
pub trait SnapshotStore {
    /// # Parameters
    /// - `snapshot`: Operation input.
    /// - `entity`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable.
    /// Resolves an entity in a stored snapshot.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, SnapshotError>;
    /// # Parameters
    /// - `base`: Operation input.
    /// - `changes`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable.
    /// Applies storage changes and returns the resulting snapshot.
    fn apply_changes(
        &self,
        base: SnapshotId,
        changes: &[SnapshotChange],
    ) -> Result<SnapshotId, SnapshotError>;
    /// # Parameters
    /// - `snapshot`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable.
    /// Iterates a stored snapshot deterministically.
    #[allow(clippy::iter_not_returning_iterator)]
    fn iter(&self, snapshot: SnapshotId) -> Result<Vec<(EntityId, RevisionId)>, SnapshotError>;
    /// # Parameters
    /// - `from`: Operation input.
    /// - `to`: Operation input.
    ///
    /// # Returns
    /// The requested snapshot result.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable.
    /// Computes exact mapping changes between stored snapshots.
    fn diff_snapshots(
        &self,
        from: SnapshotId,
        to: SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError>;
}
