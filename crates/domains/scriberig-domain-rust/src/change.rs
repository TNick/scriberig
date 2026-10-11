//! Defines Rust-specific explanations attached to shared migration operations.

use scriberig_core::EntityId;

/// One local Rust semantic change detected during reconciliation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustChange {
    /// One statement or expression changed in a function body.
    ReplaceStatement {
        /// Function entity containing the change.
        function: EntityId,
        /// Statement index.
        index: usize,
    },
    /// A statement was inserted.
    InsertStatement {
        /// Function entity containing the insertion.
        function: EntityId,
        /// Candidate statement index.
        index: usize,
    },
    /// A statement was removed.
    RemoveStatement {
        /// Function entity containing the removal.
        function: EntityId,
        /// Base statement index.
        index: usize,
    },
    /// A function or declaration moved in its parent module.
    MoveItem {
        /// Moved entity identity.
        entity: EntityId,
        /// New semantic position.
        position: usize,
    },
    /// A source-authored comment changed.
    CommentChange,
}
