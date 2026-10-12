//! Builds immutable radix trees from final snapshot mappings.

use std::sync::Arc;

use scriberig_core::{EntityId, RevisionId};

use super::{Node, nibble};

/// Builds one canonical radix subtree from entity-sorted entries.
///
/// # Parameters
/// - `entries`: Entity/revision pairs in ascending entity order.
/// - `depth`: Current radix depth.
/// - `crate`: Internal visibility marker.
/// - `RevisionId`: Revision component of each entry.
///
/// # Returns
/// A persistent radix subtree containing all entries.
pub(crate) fn build(entries: &[(EntityId, RevisionId)], depth: usize) -> Arc<Node> {
    if entries.is_empty() {
        return Arc::new(Node::Empty);
    }
    if entries.len() == 1 || depth == 32 {
        let (entity, revision) = entries[0];
        return Arc::new(Node::Leaf(entity, revision));
    }
    let mut children = Node::empty_branch();
    let mut start = 0;
    while start < entries.len() {
        let index = nibble(entries[start].0, depth);
        let mut end = start + 1;
        while end < entries.len() && nibble(entries[end].0, depth) == index {
            end += 1;
        }
        children[index] = Some(build(&entries[start..end], depth + 1));
        start = end;
    }
    Arc::new(Node::Branch(children))
}
