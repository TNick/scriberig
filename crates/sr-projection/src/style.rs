//! Defines exact universal references to projection style inputs.

use scriberig_core::{EntityId, RevisionId};

/// Identifies an immutable style revision or exact adapter-supplied value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StyleRef {
    /// References a style entity at one immutable revision.
    Revision {
        /// Stable style entity identity.
        entity: EntityId,
        /// Exact immutable style revision.
        revision: RevisionId,
    },
    /// Supplies exact serialized style bytes owned by an adapter.
    Value(StyleValue),
}

/// Carries an exact adapter-defined style value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StyleValue(Vec<u8>);

// Keep style values immutable after construction.
impl StyleValue {
    /// Creates an exact style value from its canonical bytes.
    ///
    /// # Parameters
    /// - `bytes`: Canonical adapter-defined style representation.
    ///
    /// # Returns
    /// The exact style value.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Borrows the exact style representation.
    ///
    /// # Parameters
    /// - `self`: Value to inspect.
    ///
    /// # Returns
    /// The canonical style bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
