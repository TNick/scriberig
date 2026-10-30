//! Defines persistent radix nodes used by immutable snapshots.

use std::sync::Arc;

use scriberig_core::{EncodingVersion, EntityId, RevisionId};

/// Private persistent radix node shared by related snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Node {
    Empty,
    Leaf(EntityId, RevisionId),
    Branch(Vec<Option<Arc<Node>>>),
}

impl Node {
    /// Creates the empty child set used by a radix branch.
    ///
    /// # Returns
    /// The empty sixteen-way branch.
    #[must_use]
    pub(super) fn empty_branch() -> Vec<Option<Arc<Node>>> {
        vec![None; 16]
    }

    /// Computes the canonical hash of one radix node.
    ///
    /// - `version`: Canonical encoding version included in the digest.
    ///
    /// # Returns
    /// The 32-byte content hash of the node.
    #[must_use]
    pub(super) fn hash(&self, version: EncodingVersion) -> [u8; 32] {
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
