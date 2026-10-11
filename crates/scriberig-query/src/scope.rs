//! Defines exact-snapshot query scopes.

use scriberig_core::{EntityId, SnapshotId};

/// Restricts a query to one exact canonical snapshot and semantic scope.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum QueryScope {
    /// The complete graph visible in one snapshot.
    Snapshot(SnapshotId),
    /// One entity in one snapshot.
    Entity {
        /// Exact snapshot context.
        snapshot: SnapshotId,
        /// Entity included in the scope.
        entity: EntityId,
    },
    /// An explicit set of entities in one snapshot.
    ExplicitEntities {
        /// Exact snapshot context.
        snapshot: SnapshotId,
        /// Entities included in the scope.
        entities: Vec<EntityId>,
    },
}

impl QueryScope {
    /// Returns the exact snapshot bound to this scope.
    ///
    /// # Parameters
    /// - `self`: Scope to inspect.
    ///
    /// # Returns
    /// The exact snapshot used by every query in the scope.
    #[must_use]
    pub const fn snapshot(&self) -> SnapshotId {
        match self {
            Self::Snapshot(snapshot)
            | Self::Entity { snapshot, .. }
            | Self::ExplicitEntities { snapshot, .. } => *snapshot,
        }
    }
}
