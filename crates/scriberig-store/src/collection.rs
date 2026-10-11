//! Defines immutable ordered collections used by canonical semantic state.

use std::sync::Arc;

use scriberig_core::{CanonicalEncode, CanonicalWriter};

/// A private persistent rope node.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Node<T> {
    /// Represents the end of a sequence.
    Empty,
    /// Stores one sequence value.
    Leaf(T),
    /// Shares two ordered child subtrees.
    Branch {
        /// Left subtree.
        left: Arc<Node<T>>,
        /// Right subtree.
        right: Arc<Node<T>>,
        /// Number of values in both subtrees.
        len: usize,
    },
}

/// An immutable ordered sequence whose child identity is independent of position.
#[derive(Debug, Eq, PartialEq)]
pub struct Collection<T> {
    root: Arc<Node<T>>,
    len: usize,
}

impl<T> Clone for Collection<T> {
    /// Clones the collection handle while sharing its persistent root.
    ///
    /// # Returns
    /// A new handle to the same persistent collection structure.
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            len: self.len,
        }
    }
}

/// Iterates over an immutable collection without exposing its representation.
pub struct Iter<'a, T> {
    stack: Vec<&'a Node<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    /// Returns the next collection value.
    ///
    /// # Returns
    /// The next value, or `None` when the collection is exhausted.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                Node::Empty => {}
                Node::Leaf(value) => return Some(value),
                Node::Branch { left, right, .. } => {
                    self.stack.push(right.as_ref());
                    self.stack.push(left.as_ref());
                }
            }
        }
        None
    }
}

// Returns the number of values represented by one private node.
fn node_len<T>(node: &Node<T>) -> usize {
    match node {
        Node::Empty => 0,
        Node::Leaf(_) => 1,
        Node::Branch { len, .. } => *len,
    }
}

// Joins two nodes while collapsing empty sides.
fn join<T>(left: Arc<Node<T>>, right: Arc<Node<T>>) -> Arc<Node<T>> {
    match (left.as_ref(), right.as_ref()) {
        (Node::Empty, _) => right,
        (_, Node::Empty) => left,
        _ => Arc::new(Node::Branch {
            len: node_len(left.as_ref()) + node_len(right.as_ref()),
            left,
            right,
        }),
    }
}

// Builds a balanced persistent rope from owned values.
fn from_values<T>(values: Vec<T>) -> Collection<T> {
    fn build<T>(values: Vec<T>) -> Arc<Node<T>> {
        match values.len() {
            0 => Arc::new(Node::Empty),
            1 => Arc::new(Node::Leaf(values.into_iter().next().expect("one value"))),
            length => {
                let split = length / 2;
                let mut values = values;
                let right = values.split_off(split);
                join(build(values), build(right))
            }
        }
    }

    let len = values.len();
    Collection {
        root: build(values),
        len,
    }
}

// Inserts a leaf while rebuilding only the path to its position.
fn insert_at<T>(node: &Arc<Node<T>>, index: usize, value: T) -> Arc<Node<T>> {
    match node.as_ref() {
        Node::Empty => Arc::new(Node::Leaf(value)),
        Node::Leaf(_) if index == 0 => join(Arc::new(Node::Leaf(value)), node.clone()),
        Node::Leaf(_) => join(node.clone(), Arc::new(Node::Leaf(value))),
        Node::Branch { left, right, .. } => {
            let left_len = node_len(left.as_ref());
            if index <= left_len {
                join(insert_at(left, index, value), right.clone())
            } else {
                join(left.clone(), insert_at(right, index - left_len, value))
            }
        }
    }
}

// Removes a leaf while rebuilding only the path to its position.
fn remove_at<T>(node: &Arc<Node<T>>, index: usize) -> Arc<Node<T>> {
    match node.as_ref() {
        Node::Empty => node.clone(),
        Node::Leaf(_) => Arc::new(Node::Empty),
        Node::Branch { left, right, .. } => {
            let left_len = node_len(left.as_ref());
            if index < left_len {
                join(remove_at(left, index), right.clone())
            } else {
                join(left.clone(), remove_at(right, index - left_len))
            }
        }
    }
}

impl<T> Collection<T> {
    /// Creates an empty collection.
    ///
    /// # Returns
    /// An empty persistent collection.
    #[must_use]
    pub fn create() -> Self {
        Self {
            root: Arc::new(Node::Empty),
            len: 0,
        }
    }

    /// Returns the number of values.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the collection has no values.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a value by position.
    ///
    /// # Parameters
    /// - `index`: Zero-based position to read.
    ///
    /// # Returns
    /// The value at `index`, when present.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        let mut node = self.root.as_ref();
        let mut position = index;
        loop {
            match node {
                Node::Leaf(value) if position == 0 => return Some(value),
                Node::Empty | Node::Leaf(_) => return None,
                Node::Branch { left, right, .. } => {
                    let left_len = node_len(left.as_ref());
                    if position < left_len {
                        node = left.as_ref();
                    } else {
                        position -= left_len;
                        node = right.as_ref();
                    }
                }
            }
        }
    }

    /// Iterates values in their canonical order.
    ///
    /// # Returns
    /// An iterator over values in canonical order.
    #[must_use]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            stack: vec![self.root.as_ref()],
        }
    }

    /// Inserts a value while sharing unchanged subtrees.
    ///
    /// # Parameters
    /// - `index`: Position at which to insert, clamped to the collection length.
    /// - `value`: Value to insert.
    ///
    /// # Returns
    /// A successor collection with the inserted value.
    #[must_use]
    pub fn insert(&self, index: usize, value: T) -> Self {
        let position = index.min(self.len);
        Self {
            root: insert_at(&self.root, position, value),
            len: self.len + 1,
        }
    }

    /// Removes a value while sharing unchanged subtrees.
    ///
    /// # Parameters
    /// - `index`: Position to remove.
    ///
    /// # Returns
    /// A successor collection, or an unchanged clone when `index` is absent.
    #[must_use]
    pub fn remove(&self, index: usize) -> Self {
        if index >= self.len {
            return self.clone();
        }
        Self {
            root: remove_at(&self.root, index),
            len: self.len - 1,
        }
    }

    /// Moves one value to a position in the resulting sequence.
    ///
    /// # Parameters
    /// - `from`: Current zero-based position.
    /// - `to`: Desired zero-based position after removal.
    ///
    /// # Returns
    /// A successor collection with the value moved, or an unchanged clone when
    /// `from` is absent.
    #[must_use]
    pub fn move_item(&self, from: usize, to: usize) -> Self
    where
        T: Clone,
    {
        let Some(value) = self.get(from).cloned() else {
            return self.clone();
        };
        let without = self.remove(from);
        without.insert(to.min(without.len()), value)
    }

    /// Replaces the sequence with a new ordered sequence.
    ///
    /// # Parameters
    /// - `values`: Values in their desired order.
    ///
    /// # Returns
    /// A collection containing exactly `values`.
    #[must_use]
    pub fn replace(&self, values: impl IntoIterator<Item = T>) -> Self {
        from_values(values.into_iter().collect())
    }
}

impl<T> CanonicalEncode for Collection<T>
where
    T: CanonicalEncode,
{
    /// Encodes an ordered collection without exposing its persistent layout.
    ///
    /// # Parameters
    /// - `self`: Ordered collection to encode.
    /// - `writer`: Canonical byte output.
    fn encode_canonical(&self, writer: &mut CanonicalWriter) {
        writer.write_tag(b"scriberig.collection.ordered.v1");
        writer.write_sequence_len(self.len);
        for value in self {
            value.encode_canonical(writer);
        }
    }
}

impl<'a, T> IntoIterator for &'a Collection<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    /// Returns an iterator over the collection values.
    ///
    /// # Returns
    /// An iterator over values in canonical order.
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{Collection, Node};
    use std::sync::Arc;

    /// Confirms an insertion preserves the requested ordered sequence.
    #[test]
    fn insert_preserves_ordered_values() {
        let collection = Collection::create().replace([1, 3]);
        let inserted = collection.insert(1, 2);

        assert_eq!(inserted.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    /// Confirms reordering changes collection state without changing members.
    #[test]
    fn replacement_changes_order_but_preserves_members() {
        let collection = Collection::create().replace([1, 2, 3]);
        let reordered = collection.replace([3, 2, 1]);

        assert_ne!(collection, reordered);
        assert_eq!(
            collection.iter().copied().collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(reordered.iter().copied().collect::<Vec<_>>(), vec![3, 2, 1]);
    }

    /// Confirms successor collections share an unchanged subtree internally.
    #[test]
    fn insert_shares_an_unchanged_subtree() {
        let collection = Collection::create().replace([1, 2, 3, 4]);
        let inserted = collection.insert(0, 9);

        let Node::Branch {
            right: base_right, ..
        } = collection.root.as_ref()
        else {
            panic!("expected a branch root");
        };
        let Node::Branch {
            right: inserted_right,
            ..
        } = inserted.root.as_ref()
        else {
            panic!("expected a branch root");
        };
        assert!(Arc::ptr_eq(base_right, inserted_right));
    }
}
